use alloy_rpc_types::Filter;
use alloy_sol_types::SolEvent;
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager, primitive::POOL_MANAGER_ADDRESS
};

use crate::types::utils::chunk_blocks;

pub(crate) fn historical_pool_manager_swap_filter(
    start_block: Option<u64>,
    end_block: Option<u64>
) -> Vec<Filter> {
    let swap_event = PoolManager::Swap::SIGNATURE_HASH;
    let pool_manager = *POOL_MANAGER_ADDRESS
        .get()
        .expect("POOL_MANAGER_ADDRESS has not been set");

    chunk_blocks(start_block, end_block)
        .into_iter()
        .map(|(s, e)| {
            Filter::new()
                .event_signature(swap_event)
                .address(pool_manager)
                .from_block(s)
                .to_block(e)
        })
        .collect()
}

pub(crate) fn historical_pool_manager_modify_liquidity_filter(
    start_block: Option<u64>,
    end_block: Option<u64>
) -> Vec<Filter> {
    let modify_liquidity_event = PoolManager::ModifyLiquidity::SIGNATURE_HASH;
    let pool_manager = *POOL_MANAGER_ADDRESS
        .get()
        .expect("POOL_MANAGER_ADDRESS has not been set");

    chunk_blocks(start_block, end_block)
        .into_iter()
        .map(|(s, e)| {
            Filter::new()
                .event_signature(modify_liquidity_event)
                .address(pool_manager)
                .from_block(s)
                .to_block(e)
        })
        .collect()
}
