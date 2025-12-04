use alloy_eips::BlockNumberOrTag;
use alloy_rpc_types::Filter;
use alloy_sol_types::SolEvent;
use angstrom_types_primitives::{
    ANGSTROM_DEPLOYED_BLOCK, contract_bindings::pool_manager::PoolManager,
    primitive::POOL_MANAGER_ADDRESS
};

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

pub(crate) fn chunk_blocks(
    start_block: Option<u64>,
    end_block: Option<u64>
) -> Vec<(BlockNumberOrTag, BlockNumberOrTag)> {
    let mut start_block = start_block.unwrap_or_else(|| *ANGSTROM_DEPLOYED_BLOCK.get().unwrap());
    if let Some(eb) = end_block {
        let mut tags = Vec::new();
        while eb - start_block > 1000 {
            tags.push((start_block.into(), (start_block + 1000).into()));
            start_block += 1000;
        }
        tags.push((start_block.into(), eb.into()));
        tags
    } else {
        vec![(start_block.into(), BlockNumberOrTag::Latest)]
    }
}
