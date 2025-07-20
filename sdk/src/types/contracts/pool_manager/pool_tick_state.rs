use std::collections::HashMap;

use alloy_primitives::{Address, B256, U256, aliases::I24, keccak256};
use alloy_sol_types::SolValue;
use angstrom_types::primitive::PoolId;
use futures::StreamExt;

use crate::types::{
    StorageSlotFetcher,
    contracts::{
        TickData,
        pool_manager::{
            pool_state::pool_manager_pool_state_slot,
            tick_bitmap::{next_tick_ge, normalize_tick, tick_initialized}
        },
        utils::{max_valid_tick, min_valid_tick}
    }
};

// tick state
pub const POOL_MANAGER_POOL_TICK_OFFSET_SLOT: u8 = 4;
pub const POOL_MANAGER_POOL_TICK_BITMAP_OFFSET_SLOT: u8 = 5;
pub const POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE0_X128_SLOT_OFFSET: u8 = 1;
pub const POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE1_X128_SLOT_OFFSET: u8 = 2;

pub fn pool_manager_pool_tick_slot(pool_id: U256, tick: I24) -> B256 {
    let inner = U256::from_be_bytes(pool_manager_pool_state_slot(pool_id).0)
        + U256::from(POOL_MANAGER_POOL_TICK_OFFSET_SLOT);
    keccak256((tick, inner).abi_encode())
}

pub fn pool_manager_pool_tick_bitmap_slot(pool_id: U256, word_position: i16) -> B256 {
    let inner = U256::from_be_bytes(pool_manager_pool_state_slot(pool_id).0)
        + U256::from(POOL_MANAGER_POOL_TICK_BITMAP_OFFSET_SLOT);
    keccak256((word_position, inner).abi_encode())
}

pub async fn pool_manager_pool_tick_fee_growth_outside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    tick: I24
) -> eyre::Result<(U256, U256)> {
    let pool_tick_slot = pool_manager_pool_tick_slot(pool_id.into(), tick);
    let pool_tick_slot_base = U256::from_be_slice(pool_tick_slot.as_slice());

    let fee_growth_outside0_x128_slot = pool_tick_slot_base
        + U256::from(POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE0_X128_SLOT_OFFSET);
    let fee_growth_outside1_x128_slot = pool_tick_slot_base
        + U256::from(POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE1_X128_SLOT_OFFSET);

    let (fee_growth_outside0_x128, fee_growth_outside1_x128) = tokio::try_join!(
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_outside0_x128_slot.into(),
            block_number
        ),
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_outside1_x128_slot.into(),
            block_number
        )
    )?;

    Ok((fee_growth_outside0_x128, fee_growth_outside1_x128))
}

pub async fn pool_manager_load_tick_map<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    tick_spacing: I24,
    start_tick: Option<I24>,
    end_tick: Option<I24>
) -> eyre::Result<HashMap<I24, TickData>> {
    let start_tick = start_tick
        .map(|t| normalize_tick(t, tick_spacing))
        .unwrap_or(min_valid_tick(tick_spacing));
    let end_tick = end_tick
        .map(|t| normalize_tick(t, tick_spacing))
        .unwrap_or(max_valid_tick(tick_spacing));

    let mut ct = start_tick;
    let mut initialized_ticks = Vec::new();
    while ct <= end_tick {
        let (_, tick) = next_tick_ge(
            slot_fetcher,
            pool_manager_address,
            block_number,
            tick_spacing,
            pool_id,
            ct,
            true
        )
        .await?;
        initialized_ticks.push(tick);
        ct = tick;
    }

    let mut tick_data_loading_stream = futures::stream::iter(initialized_ticks)
        .map(async |tick| {
            let tick = I24::unchecked_from(tick);

            pool_manager_load_tick_data(
                slot_fetcher,
                pool_manager_address,
                block_number,
                tick_spacing,
                pool_id,
                tick
            )
            .await
            .map(|d| (tick, d))
        })
        .buffer_unordered(1000);

    let mut loaded_tick_data = HashMap::new();
    while let Some(val) = tick_data_loading_stream.next().await {
        let (k, v) = val?;
        loaded_tick_data.insert(k, v);
    }

    Ok(loaded_tick_data)
}

pub async fn pool_manager_load_tick_data<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    tick_spacing: I24,
    pool_id: PoolId,
    tick: I24
) -> eyre::Result<TickData> {
    let pool_tick_slot = pool_manager_pool_tick_slot(pool_id.into(), tick);
    let pool_tick_slot_base = U256::from_be_slice(pool_tick_slot.as_slice());

    let liquidity_slot = pool_tick_slot_base;
    let fee_growth_outside0_x128_slot = pool_tick_slot_base
        + U256::from(POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE0_X128_SLOT_OFFSET);
    let fee_growth_outside1_x128_slot = pool_tick_slot_base
        + U256::from(POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE1_X128_SLOT_OFFSET);

    let (liquidity, fee_growth_outside0_x128, fee_growth_outside1_x128, is_initialized) = tokio::try_join!(
        slot_fetcher.storage_at(pool_manager_address, liquidity_slot.into(), block_number),
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_outside0_x128_slot.into(),
            block_number
        ),
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_outside1_x128_slot.into(),
            block_number
        ),
        tick_initialized(
            slot_fetcher,
            pool_manager_address,
            block_number,
            tick_spacing,
            pool_id,
            tick
        )
    )?;

    let liquidity_bytes: [u8; 32] = liquidity.to_be_bytes();

    Ok(TickData {
        tick,
        is_initialized,
        liquidity_net: i128::from_be_bytes(liquidity_bytes[..16].try_into().unwrap()),
        liquidity_gross: u128::from_be_bytes(liquidity_bytes[16..].try_into().unwrap()),
        fee_growth_outside0_x128,
        fee_growth_outside1_x128
    })
}

#[cfg(test)]
mod tests {
    use angstrom_types::{self, primitive::POOL_MANAGER_ADDRESS};

    use super::*;
    use crate::{
        apis::AngstromDataApi,
        test_utils::valid_test_params::init_valid_position_params_with_provider
    };

    #[tokio::test]
    async fn test_pool_manager_pool_tick_fee_growth_outside() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let results = pool_manager_pool_tick_fee_growth_outside(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into(),
            pos_info.current_pool_tick
        )
        .await
        .unwrap();

        println!("{results:?}");

        // assert_eq!(results, pos_info.position_liquidity);
    }

    #[tokio::test]
    async fn test_pool_manager_load_tick_map() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let tick_spacing = pos_info.pool_key.tickSpacing;
        let pool_id = pos_info.pool_key.into();

        let results = pool_manager_load_tick_map(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pool_id,
            tick_spacing,
            None,
            None
        )
        .await
        .unwrap();

        assert_eq!(results.len(), 8);
    }

    #[tokio::test]
    async fn test_pool_manager_load_tick_data() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let (_, pool_info) = provider
            .pool_data_by_pool_id(pos_info.pool_key.clone().into(), Some(block_number))
            .await
            .unwrap();

        let tick_spacing = pos_info.pool_key.tickSpacing;
        let pool_id = pos_info.pool_key.into();
        for (tick_actual, tick_actual_data) in pool_info.ticks {
            let results = pool_manager_load_tick_data(
                &provider,
                *POOL_MANAGER_ADDRESS.get().unwrap(),
                Some(block_number),
                tick_spacing,
                pool_id,
                I24::unchecked_from(tick_actual)
            )
            .await
            .unwrap();

            assert_eq!(results.is_initialized, tick_actual_data.initialized);
            assert_eq!(results.liquidity_gross, tick_actual_data.liquidity_gross);
            assert_eq!(results.liquidity_net, tick_actual_data.liquidity_net);
        }
    }
}
