use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::{Address, U256, aliases::I24};
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager::PoolKey, primitive::PoolId
};
use uniswap_storage::v4::{UnpackedPositionInfo, V4UserLiquidityPosition};

use super::data_api::AngstromL2DataApi;
use crate::{l2::AngstromL2Chain, types::fees::LiquidityPositionFees};

#[async_trait::async_trait]
pub trait AngstromL2UserApi<N: Network>: AngstromL2DataApi<N> {
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)>;

    async fn position_liquidity(
        &self,
        position_token_id: U256,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<u128>;

    async fn all_user_positions(
        &self,
        owner: Address,
        start_token_id: U256,
        last_token_id: U256,
        pool_id: Option<PoolId>,
        max_results: Option<usize>,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<V4UserLiquidityPosition>>;

    async fn user_position_fees(
        &self,
        position_token_id: U256,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<LiquidityPositionFees>;

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
    ) -> eyre::Result<U256>;
}
