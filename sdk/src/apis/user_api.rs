use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use alloy_sol_types::SolType;
use angstrom_types::{
    contract_bindings::position_manager::PositionManager::{self, PoolKey},
    primitive::{ANGSTROM_ADDRESS, POSITION_MANAGER_ADDRESS}
};
use revm::{
    Context, ExecuteEvm, MainBuilder, MainContext,
    context::{BlockEnv, result::ExecutionResult}
};
use revm_database::{AlloyDB, CacheDB, WrapDatabaseAsync};

use super::{data_api::AngstromDataApi, utils::*};
use crate::types::{
    UserLiquidityPosition,
    contract_bindings::UserPositionFetcher::{self, AllUserPositions},
    positions::{UnpackPositionInfo, UnpackedPositionInfo}
};

#[async_trait::async_trait]
pub trait AngstromUserApi: AngstromDataApi {
    async fn position_and_pool_info_by_token_id(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)>;

    async fn position_liquidity_by_token_id(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<u128>;

    async fn all_user_positions(
        &self,
        owner: Address,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<UserLiquidityPosition>>;
}

#[async_trait::async_trait]
impl<P: Provider> AngstromUserApi for P {
    async fn position_and_pool_info_by_token_id(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)> {
        let call_ret = view_call(
            &self,
            block_number,
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            PositionManager::getPoolAndPositionInfoCall { tokenId: position_token_id }
        )
        .await??;

        Ok((call_ret.poolKey, call_ret.info.unpack_position_info()))
    }

    async fn position_liquidity_by_token_id(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<u128> {
        Ok(view_call(
            &self,
            block_number,
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            PositionManager::getPositionLiquidityCall { tokenId: position_token_id }
        )
        .await??)
    }

    async fn all_user_positions(
        &self,
        owner: Address,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        let block_number =
            if let Some(b) = block_number { b } else { self.get_block_number().await? };

        let deployer = UserPositionFetcher::deploy_builder(
            self.clone(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            *ANGSTROM_ADDRESS.get().unwrap(),
            owner
        )
        .block(block_number.into());

        let deploy_tx = deployer.clone().into_transaction_request();

        let evm_cache = CacheDB::new(
            WrapDatabaseAsync::new(AlloyDB::new(self.clone(), block_number.into())).unwrap()
        );
        let mut revm = Context::mainnet()
            .with_block(BlockEnv { number: U256::from(block_number), ..Default::default() })
            .with_db(evm_cache)
            .build_mainnet();
        revm.ctx = revm
            .ctx
            .modify_tx_chained(|tx| {
                tx.tx_type = deploy_tx.transaction_type.unwrap_or_default();
                tx.caller = deploy_tx.from.unwrap_or_default();
                tx.gas_limit = u64::MAX;
                tx.gas_price = deploy_tx.gas_price.unwrap_or_default();
                tx.kind = deploy_tx.kind().unwrap_or_default();
                tx.value = deploy_tx.value.unwrap_or_default();
                tx.data = deploy_tx
                    .input
                    .data
                    .unwrap_or(deploy_tx.input.input.unwrap_or_default());
                tx.nonce = deploy_tx.nonce.unwrap_or_default();
                tx.chain_id = deploy_tx.chain_id;
            })
            .modify_block_chained(|block| block.gas_limit = u64::MAX)
            .modify_cfg_chained(|cfg| cfg.limit_contract_code_size = Some(u64::MAX as usize));

        let out = revm.transact(revm.ctx.tx.clone())?;

        let positions = match out.result {
            ExecutionResult::Revert { output, .. } => {
                AllUserPositions::abi_decode(&output)?.positions;
            }
            _ => eyre::bail!("failed to deploy UserPositionFetcher contract - {out:?}")
        };

        Ok(positions.into_iter().map(Into::into).collect())
    }
}

#[cfg(test)]
mod tests {

    use crate::{
        apis::AngstromUserApi,
        test_utils::valid_test_params::init_valid_position_params_with_provider
    };

    #[tokio::test]
    async fn test_position_and_pool_info_by_token_id() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let (pool_key, unpacked_position_info) = provider
            .position_and_pool_info_by_token_id(pos_info.position_token_id, Some(block_number))
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
            .position_liquidity_by_token_id(pos_info.position_token_id, Some(block_number))
            .await
            .unwrap();

        assert_eq!(pos_info.position_liquidity, position_liquidity);
    }

    #[tokio::test]
    async fn test_all_user_positions() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let position_liquidity = provider
            .all_user_positions(pos_info.owner, Some(block_number))
            .await
            .unwrap();

        println!("{:?}", position_liquidity);

        // assert_eq!(pos_info.position_liquidity, position_liquidity);
    }
}
