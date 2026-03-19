use alloy_eips::BlockNumberOrTag;
use alloy_primitives::Address;
use alloy_rpc_types::Filter;
use alloy_sol_types::SolEvent;
use angstrom_types_primitives::contract_bindings::pool_manager::PoolManager;

pub(crate) fn historical_pool_manager_swap_filter(
    start_block: Option<u64>,
    end_block: Option<u64>,
    pool_manager_address: Address,
    deploy_block: u64
) -> Vec<Filter> {
    let swap_event = PoolManager::Swap::SIGNATURE_HASH;

    chunk_blocks(start_block, end_block, deploy_block)
        .into_iter()
        .map(|(s, e)| {
            Filter::new()
                .event_signature(swap_event)
                .address(pool_manager_address)
                .from_block(s)
                .to_block(e)
        })
        .collect()
}

pub(crate) fn historical_pool_manager_modify_liquidity_filter(
    start_block: Option<u64>,
    end_block: Option<u64>,
    pool_manager_address: Address,
    deploy_block: u64
) -> Vec<Filter> {
    let modify_liquidity_event = PoolManager::ModifyLiquidity::SIGNATURE_HASH;

    chunk_blocks(start_block, end_block, deploy_block)
        .into_iter()
        .map(|(s, e)| {
            Filter::new()
                .event_signature(modify_liquidity_event)
                .address(pool_manager_address)
                .from_block(s)
                .to_block(e)
        })
        .collect()
}

pub(crate) fn chunk_blocks(
    start_block: Option<u64>,
    end_block: Option<u64>,
    deploy_block: u64
) -> Vec<(BlockNumberOrTag, BlockNumberOrTag)> {
    let mut start_block = start_block.unwrap_or(deploy_block);
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

pub(crate) fn split_filter_by_blocks(filter: &Filter) -> Option<(Filter, Filter)> {
    let (start_block, end_block) = (
        filter.block_option.get_from_block()?.as_number()?,
        filter.block_option.get_to_block()?.as_number()?
    );

    if start_block >= end_block {
        return None;
    }

    let midpoint = start_block + (end_block - start_block).div_ceil(2);

    let filter_a = filter
        .clone()
        .from_block(start_block)
        .to_block(midpoint - 1);
    let filter_b = filter.clone().from_block(midpoint).to_block(end_block);

    Some((filter_a, filter_b))
}
