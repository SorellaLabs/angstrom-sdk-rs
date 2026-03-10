use alloy_eips::BlockId;
use alloy_primitives::{Address, U256, aliases::I24};
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager::PoolKey, primitive::PoolId
};
use auto_impl::auto_impl;
use uniswap_storage::v4::{UnpackedPositionInfo, V4UserLiquidityPosition};

use super::data_api::AngstromL1DataApi;
use crate::{l1::AngstromL1Chain, types::fees::LiquidityPositionFees};

#[async_trait::async_trait]
#[auto_impl(&, Box, Arc)]
pub trait AngstromL1UserApi: AngstromL1DataApi {
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)>;

    async fn position_liquidity(
        &self,
        position_token_id: U256,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<u128>;

    async fn all_user_positions(
        &self,
        owner: Address,
        start_token_id: U256,
        last_token_id: U256,
        pool_id: Option<PoolId>,
        max_results: Option<usize>,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<V4UserLiquidityPosition>>;

    async fn user_position_fees(
        &self,
        position_token_id: U256,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<LiquidityPositionFees>;

    async fn angstrom_fees(
        &self,
        pool_id: PoolId,
        current_pool_tick: I24,
        position_token_id: U256,
        tick_lower: I24,
        tick_upper: I24,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<U256>;
}
