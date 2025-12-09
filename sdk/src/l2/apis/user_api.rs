use alloy_network::Network;
use alloy_primitives::{Address, U256};
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager::PoolKey, primitive::PoolId
};
use uniswap_storage::v4::{UnpackedPositionInfo, V4UserLiquidityPosition};

use super::data_api::AngstromL2DataApi;
use crate::{l2::constants::AngstromL2Chain, types::fees::LiquidityPositionFees};

#[async_trait::async_trait]
pub trait AngstromL2UserApi<N: Network>: AngstromL2DataApi<N> {
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)>;

    async fn position_liquidity(
        &self,
        position_token_id: U256,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<u128>;

    async fn all_user_positions(
        &self,
        owner: Address,
        start_token_id: U256,
        last_token_id: U256,
        pool_id: Option<PoolId>,
        max_results: Option<usize>,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<V4UserLiquidityPosition>>;

    async fn user_position_fees(
        &self,
        position_token_id: U256,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<LiquidityPositionFees>;
}
