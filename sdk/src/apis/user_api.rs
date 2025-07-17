use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use angstrom_types::{
    contract_bindings::pool_manager::PoolManager::PoolKey,
    primitive::{ANGSTROM_ADDRESS, POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS, PoolId}
};

use super::data_api::AngstromDataApi;
use crate::types::positions::{
    UnpackedPositionInfo, UserLiquidityPosition,
    fees::{LiquidityPositionFees, position_fees},
    pool_manager_position_state_liquidity, position_manager_next_token_id,
    position_manager_owner_of, position_manager_pool_key_and_info
};

#[async_trait::async_trait]
pub trait AngstromUserApi: AngstromDataApi {
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)>;

    async fn position_liquidity(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<u128>;

    async fn all_user_positions(
        &self,
        owner: Address,
        start_token_id: U256,
        last_token_id: U256,
        pool_id: Option<PoolId>,
        max_results: Option<usize>,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<UserLiquidityPosition>>;

    async fn user_position_fees(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<LiquidityPositionFees>;
}

#[async_trait::async_trait]
impl<P: Provider> AngstromUserApi for P {
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)> {
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            self.root(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            block_number,
            position_token_id
        )
        .await?;

        Ok((pool_key, position_info))
    }

    async fn position_liquidity(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<u128> {
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            self.root(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            block_number,
            position_token_id
        )
        .await?;

        let liquidity = pool_manager_position_state_liquidity(
            self.root(),
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            block_number,
            pool_key.into(),
            position_token_id,
            position_info.tick_lower,
            position_info.tick_upper
        )
        .await?;

        Ok(liquidity)
    }

    async fn all_user_positions(
        &self,
        owner: Address,
        mut start_token_id: U256,
        mut end_token_id: U256,
        pool_id: Option<PoolId>,
        max_results: Option<usize>,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        let position_manager_address = *POSITION_MANAGER_ADDRESS.get().unwrap();
        let pool_manager_address = *POOL_MANAGER_ADDRESS.get().unwrap();
        let angstrom_address = *ANGSTROM_ADDRESS.get().unwrap();

        let root = self.root();

        if start_token_id == U256::ZERO {
            start_token_id = U256::from(1u8);
        }

        if end_token_id == U256::ZERO {
            end_token_id =
                position_manager_next_token_id(root, position_manager_address, block_number)
                    .await?;
        }

        let mut all_positions = Vec::new();
        while start_token_id <= end_token_id {
            let owner_of = position_manager_owner_of(
                root,
                position_manager_address,
                block_number,
                start_token_id
            )
            .await?;

            if owner_of != owner {
                start_token_id += U256::from(1u8);
                continue;
            }

            let (pool_key, position_info) = position_manager_pool_key_and_info(
                root,
                position_manager_address,
                block_number,
                start_token_id
            )
            .await?;

            if pool_key.hooks != angstrom_address
                || pool_id
                    .map(|id| id != PoolId::from(pool_key.clone()))
                    .unwrap_or_default()
            {
                start_token_id += U256::from(1u8);
                continue;
            }

            let liquidity = pool_manager_position_state_liquidity(
                root,
                pool_manager_address,
                block_number,
                pool_key.clone().into(),
                start_token_id,
                position_info.tick_lower,
                position_info.tick_upper
            )
            .await?;

            all_positions.push(UserLiquidityPosition {
                token_id: start_token_id,
                tick_lower: position_info.tick_lower,
                tick_upper: position_info.tick_upper,
                liquidity,
                pool_key
            });

            if let Some(max_res) = max_results {
                if all_positions.len() >= max_res {
                    break;
                }
            }

            start_token_id += U256::from(1u8);
        }

        Ok(all_positions)
    }

    async fn user_position_fees(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<LiquidityPositionFees> {
        let ((pool_key, position_info), position_liquidity) = tokio::try_join!(
            self.position_and_pool_info(position_token_id, block_number),
            self.position_liquidity(position_token_id, block_number),
        )?;

        let pool_id = pool_key.clone().into();
        let slot0 = self.slot0_by_pool_id(pool_id, block_number).await?;

        Ok(position_fees(
            self.root(),
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            *ANGSTROM_ADDRESS.get().unwrap(),
            block_number,
            pool_id,
            slot0.tick,
            position_token_id,
            position_info.tick_lower,
            position_info.tick_upper,
            position_liquidity
        )
        .await?)
    }
}

#[cfg(test)]
mod tests {

    use alloy_primitives::U256;

    use crate::{
        apis::AngstromUserApi,
        test_utils::valid_test_params::init_valid_position_params_with_provider
    };

    #[tokio::test]
    async fn test_position_and_pool_info_by_token_id() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let (pool_key, unpacked_position_info) = provider
            .position_and_pool_info(pos_info.position_token_id, Some(block_number))
            .await
            .unwrap();

        assert_eq!(pool_key, pos_info.pool_key);
        assert_eq!(unpacked_position_info, pos_info.as_unpacked_position_info());
    }

    #[tokio::test]
    async fn test_position_liquidity_by_token_id() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let position_liquidity = provider
            .position_liquidity(pos_info.position_token_id, Some(block_number))
            .await
            .unwrap();

        assert_eq!(pos_info.position_liquidity, position_liquidity);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_all_user_positions() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let bound: u64 = 10;

        let position_liquidity = provider
            .all_user_positions(
                pos_info.owner,
                pos_info.position_token_id - U256::from(bound),
                pos_info.position_token_id + U256::from(bound),
                None,
                None,
                Some(block_number)
            )
            .await
            .unwrap();

        assert_eq!(position_liquidity.len(), 4);
    }

    #[tokio::test]
    async fn test_user_position_fees() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = provider
            .user_position_fees(pos_info.position_token_id, Some(block_number))
            .await
            .unwrap();

        println!("{results:?}");
    }
}
