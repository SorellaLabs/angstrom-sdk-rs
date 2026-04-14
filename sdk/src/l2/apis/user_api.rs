use std::collections::HashSet;

use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::{Address, B256, U256, aliases::I24};
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager::PoolKey, primitive::PoolId
};
use uniswap_storage::{
    angstrom::l2::angstrom_l2::{angstrom_l2_growth_inside, angstrom_l2_last_growth_inside},
    v4::{
        UnpackedPositionInfo, V4UserLiquidityPosition,
        pool_manager::position_state::pool_manager_position_state_liquidity,
        position_manager::{
            position_manager_next_token_id, position_manager_owner_of,
            position_manager_pool_key_and_info
        }
    }
};

use super::data_api::AngstromL2DataApi;
use crate::{
    l2::AngstromL2Chain,
    types::fees::{LiquidityPositionFees, uniswap_fee_deltas}
};

impl<P, N> AngstromL2UserApi<N> for P
where
    P: AngstromL2DataApi<N>,
    N: Network
{
}

#[async_trait::async_trait]
pub trait AngstromL2UserApi<N: Network>: AngstromL2DataApi<N> {
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)> {
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            self,
            chain.constants().uniswap_constants().position_manager(),
            block_id,
            position_token_id
        )
        .await?;

        Ok((
            PoolKey {
                currency0:   pool_key.currency0,
                currency1:   pool_key.currency1,
                fee:         pool_key.fee,
                tickSpacing: pool_key.tickSpacing,
                hooks:       pool_key.hooks
            },
            position_info
        ))
    }

    async fn position_liquidity(
        &self,
        position_token_id: U256,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<u128> {
        let consts = chain.constants();
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            self,
            consts.uniswap_constants().position_manager(),
            block_id,
            position_token_id
        )
        .await?;

        let liquidity = pool_manager_position_state_liquidity(
            self,
            consts.uniswap_constants().pool_manager(),
            consts.uniswap_constants().position_manager(),
            pool_key.into(),
            position_token_id,
            position_info.tick_lower,
            position_info.tick_upper,
            block_id
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
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<V4UserLiquidityPosition>> {
        let consts = chain.constants();

        let position_manager_address = consts.uniswap_constants().position_manager();
        let pool_manager_address = consts.uniswap_constants().pool_manager();

        let all_angstrom_hooks = if pool_id.is_none() {
            self.all_pool_keys(block_id, chain)
                .await?
                .into_iter()
                .map(|key| key.hooks)
                .collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };

        if start_token_id == U256::ZERO {
            start_token_id = U256::from(1u8);
        }

        if end_token_id == U256::ZERO {
            end_token_id =
                position_manager_next_token_id(self, position_manager_address, block_id).await?;
        }

        let mut all_positions = Vec::new();
        while start_token_id <= end_token_id {
            let owner_of =
                position_manager_owner_of(self, position_manager_address, block_id, start_token_id)
                    .await?;

            if owner_of != owner {
                start_token_id += U256::from(1u8);
                continue;
            }

            let (pool_key, position_info) = position_manager_pool_key_and_info(
                self,
                position_manager_address,
                block_id,
                start_token_id
            )
            .await?;

            if !all_angstrom_hooks.contains(&pool_key.hooks)
                || pool_id
                    .map(|id| id != B256::from(pool_key))
                    .unwrap_or_default()
            {
                start_token_id += U256::from(1u8);
                continue;
            }

            let liquidity = pool_manager_position_state_liquidity(
                self,
                pool_manager_address,
                position_manager_address,
                pool_key.into(),
                start_token_id,
                position_info.tick_lower,
                position_info.tick_upper,
                block_id
            )
            .await?;

            all_positions.push(V4UserLiquidityPosition {
                token_id: start_token_id,
                tick_lower: position_info.tick_lower,
                tick_upper: position_info.tick_upper,
                liquidity,
                pool_key
            });

            if let Some(max_res) = max_results
                && all_positions.len() >= max_res
            {
                break;
            }

            start_token_id += U256::from(1u8);
        }

        Ok(all_positions)
    }

    async fn user_position_fees(
        &self,
        position_token_id: U256,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<LiquidityPositionFees> {
        let consts = chain.constants();

        let ((pool_key, position_info), position_liquidity) = tokio::try_join!(
            self.position_and_pool_info(position_token_id, block_id, chain),
            self.position_liquidity(position_token_id, block_id, chain),
        )?;

        let hook = pool_key.hooks;
        let pool_id = pool_key.into();
        let slot0 = self.slot0_by_pool_id(pool_id, block_id, chain).await?;

        let (angstrom_fee_delta, (uniswap_token0_fee_delta, uniswap_token1_fee_delta)) = tokio::try_join!(
            self.angstrom_l2_fees(
                pool_id,
                Some(hook),
                slot0.tick,
                position_token_id,
                position_info.tick_lower,
                position_info.tick_upper,
                block_id,
                chain
            ),
            uniswap_fee_deltas(
                self,
                consts.uniswap_constants().pool_manager(),
                consts.uniswap_constants().position_manager(),
                block_id,
                pool_id,
                slot0.tick,
                position_token_id,
                position_info.tick_lower,
                position_info.tick_upper,
            )
        )?;

        Ok(LiquidityPositionFees::new(
            position_liquidity,
            angstrom_fee_delta,
            uniswap_token0_fee_delta,
            uniswap_token1_fee_delta
        ))
    }

    async fn angstrom_l2_fees(
        &self,
        pool_id: PoolId,
        hook_address: Option<Address>,
        current_pool_tick: I24,
        position_token_id: U256,
        tick_lower: I24,
        tick_upper: I24,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<U256> {
        let hook = if let Some(hook_address) = hook_address {
            hook_address
        } else {
            self.hook_by_pool_id(pool_id, block_id, chain).await?
        };
        let consts = chain.constants();
        let (growth_inside, last_growth_inside) = tokio::try_join!(
            angstrom_l2_growth_inside(
                self,
                hook,
                pool_id,
                current_pool_tick,
                tick_lower,
                tick_upper,
                block_id,
            ),
            angstrom_l2_last_growth_inside(
                self,
                hook,
                consts.uniswap_constants().position_manager(),
                pool_id,
                position_token_id,
                tick_lower,
                tick_upper,
                block_id,
            ),
        )?;

        Ok(growth_inside - last_growth_inside)
    }
}

#[cfg(test)]
mod user_api_tests {

    use alloy_primitives::U256;

    use crate::{
        l2::{
            apis::user_api::AngstromL2UserApi,
            test_utils::valid_test_params::init_valid_position_params_with_provider
        },
        types::fees::LiquidityPositionFees
    };

    #[tokio::test]
    async fn test_position_and_pool_info_by_token_id() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_for_liquidity_add + 1;

        let (pool_key, unpacked_position_info) = provider
            .position_and_pool_info(pos_info.position_token_id, block_number.into(), pos_info.chain)
            .await
            .unwrap();

        assert_eq!(pool_key, pos_info.pool_key);
        assert_eq!(unpacked_position_info, pos_info.as_unpacked_position_info());
    }

    #[tokio::test]
    async fn test_position_liquidity_by_token_id() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_for_liquidity_add + 1;

        let position_liquidity = provider
            .position_liquidity(pos_info.position_token_id, block_number.into(), pos_info.chain)
            .await
            .unwrap();

        assert_eq!(pos_info.position_liquidity, position_liquidity);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_all_user_positions() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_for_liquidity_add + 1;

        let bound: u64 = 10;

        let position_liquidity = provider
            .all_user_positions(
                pos_info.owner,
                pos_info.position_token_id - U256::from(bound),
                pos_info.position_token_id + U256::from(bound),
                None,
                None,
                block_number.into(),
                pos_info.chain
            )
            .await
            .unwrap();

        assert_eq!(position_liquidity.len(), 2);
    }

    #[tokio::test]
    async fn test_user_position_fees() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_for_liquidity_add + 100;

        let results = provider
            .user_position_fees(pos_info.position_token_id, block_number.into(), pos_info.chain)
            .await
            .unwrap();

        assert_eq!(
            results,
            LiquidityPositionFees {
                position_liquidity:   41433601053552,
                angstrom_token0_fees: U256::ZERO,
                uniswap_token0_fees:  U256::from(3143446492832_u128),
                uniswap_token1_fees:  U256::from(25_u128)
            }
        );
    }
}
