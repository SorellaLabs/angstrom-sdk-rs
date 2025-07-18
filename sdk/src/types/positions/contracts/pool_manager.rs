use std::collections::HashMap;

use alloy_primitives::{Address, B256, U256, aliases::I24, keccak256};
use alloy_sol_types::SolValue;
use angstrom_types::primitive::PoolId;
use futures::StreamExt;

use crate::types::{
    StorageSlotFetcher,
    positions::{TickData, UnpackSlot0, UnpackedSlot0, utils::*}
};

// pool state
pub const POOL_MANAGER_POOL_STATE_MAP_SLOT: u8 = 6;
pub const POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL0_X128_SLOT_OFFSET: u8 = 1;
pub const POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL1_X128_SLOT_OFFSET: u8 = 2;
pub const POOL_MANAGER_POOL_LIQUIDITY_SLOT_OFFSET: u8 = 3;

// tick state
pub const POOL_MANAGER_POOL_TICK_OFFSET_SLOT: u8 = 4;
pub const POOL_MANAGER_POOL_TICK_BITMAP_OFFSET_SLOT: u8 = 5;
pub const POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE0_X128_SLOT_OFFSET: u8 = 1;
pub const POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE1_X128_SLOT_OFFSET: u8 = 2;

// position state
pub const POOL_MANAGER_POSITION_STATE_OFFSET_SLOT: u8 = 6;
pub const POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE0_LAST_X128_SLOT_OFFSET: u8 = 1;
pub const POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE1_LAST_X128_SLOT_OFFSET: u8 = 2;

pub fn pool_manager_pool_state_slot(pool_id: U256) -> B256 {
    keccak256((pool_id, U256::from(POOL_MANAGER_POOL_STATE_MAP_SLOT)).abi_encode())
}

pub fn pool_manager_position_state_slot(pool_id: U256, position_id: U256) -> B256 {
    let pools_slot = U256::from_be_slice(pool_manager_pool_state_slot(pool_id).as_slice())
        + U256::from(POOL_MANAGER_POSITION_STATE_OFFSET_SLOT);
    keccak256((position_id, pools_slot).abi_encode())
}

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

pub async fn pool_manager_position_fee_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    current_tick: I24,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<(U256, U256)> {
    let (
        (fee_growth_global0_x128, fee_growth_global1_x128),
        (tick_lower_fee_growth_outside0_x128, tick_lower_fee_growth_outside1_x128),
        (tick_upper_fee_growth_outside0_x128, tick_upper_fee_growth_outside1_x128)
    ) = tokio::try_join!(
        pool_manager_pool_last_fee_growth_global(
            slot_fetcher,
            pool_manager_address,
            block_number,
            pool_id,
        ),
        pool_manager_pool_tick_fee_growth_outside(
            slot_fetcher,
            pool_manager_address,
            block_number,
            pool_id,
            tick_lower
        ),
        pool_manager_pool_tick_fee_growth_outside(
            slot_fetcher,
            pool_manager_address,
            block_number,
            pool_id,
            tick_upper
        )
    )?;

    let (fee_growth_inside0_x128, fee_growth_inside1_x128) = if current_tick < tick_lower {
        (
            tick_lower_fee_growth_outside0_x128 - tick_upper_fee_growth_outside0_x128,
            tick_lower_fee_growth_outside1_x128 - tick_upper_fee_growth_outside1_x128
        )
    } else if current_tick >= tick_upper {
        (
            tick_upper_fee_growth_outside0_x128 - tick_lower_fee_growth_outside0_x128,
            tick_upper_fee_growth_outside1_x128 - tick_lower_fee_growth_outside1_x128
        )
    } else {
        (
            fee_growth_global0_x128
                - tick_lower_fee_growth_outside0_x128
                - tick_upper_fee_growth_outside0_x128,
            fee_growth_global1_x128
                - tick_lower_fee_growth_outside1_x128
                - tick_upper_fee_growth_outside1_x128
        )
    };

    Ok((fee_growth_inside0_x128, fee_growth_inside1_x128))
}

pub async fn pool_manager_pool_last_fee_growth_global<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId
) -> eyre::Result<(U256, U256)> {
    let pool_state_slot = pool_manager_pool_state_slot(pool_id.into());
    let pool_state_slot_base = U256::from_be_slice(pool_state_slot.as_slice());

    let fee_growth_global0_x128_slot =
        pool_state_slot_base + U256::from(POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL0_X128_SLOT_OFFSET);
    let fee_growth_global1_x128_slot =
        pool_state_slot_base + U256::from(POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL1_X128_SLOT_OFFSET);

    let (fee_growth_global0_x128, fee_growth_global1_x128) = tokio::try_join!(
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_global0_x128_slot.into(),
            block_number
        ),
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_global1_x128_slot.into(),
            block_number
        )
    )?;

    Ok((fee_growth_global0_x128, fee_growth_global1_x128))
}

pub async fn pool_manager_pool_slot0<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId
) -> eyre::Result<UnpackedSlot0> {
    let pool_state_slot = pool_manager_pool_state_slot(pool_id.into());

    let packed_slot0 = slot_fetcher
        .storage_at(pool_manager_address, pool_state_slot, block_number)
        .await?;

    Ok(packed_slot0.unpack_slot0())
}

pub async fn pool_manager_position_state_last_fee_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<(U256, U256)> {
    let position_key = U256::from_be_slice(
        encode_position_key(position_token_id, tick_lower, tick_upper).as_slice()
    );
    let position_state_slot = pool_manager_position_state_slot(pool_id.into(), position_key);
    let position_state_slot_base = U256::from_be_slice(position_state_slot.as_slice());

    let fee_growth_inside0_last_x128_slot = position_state_slot_base
        + U256::from(POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE0_LAST_X128_SLOT_OFFSET);
    let fee_growth_inside1_last_x128_slot = position_state_slot_base
        + U256::from(POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE1_LAST_X128_SLOT_OFFSET);

    let (fee_growth_inside0_last_x128, fee_growth_inside1_last_x128) = tokio::try_join!(
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_inside0_last_x128_slot.into(),
            block_number
        ),
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_inside1_last_x128_slot.into(),
            block_number
        )
    )?;

    Ok((fee_growth_inside0_last_x128, fee_growth_inside1_last_x128))
}

pub async fn pool_manager_position_state_liquidity<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<u128> {
    let position_key = U256::from_be_slice(
        encode_position_key(position_token_id, tick_lower, tick_upper).as_slice()
    );
    let position_state_slot = pool_manager_position_state_slot(pool_id.into(), position_key);

    let liquidity = slot_fetcher
        .storage_at(pool_manager_address, position_state_slot, block_number)
        .await?;

    Ok(liquidity.to())
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

pub async fn pool_manager_pool_liquidity<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId
) -> eyre::Result<U256> {
    let pool_state_slot = pool_manager_pool_state_slot(pool_id.into());
    let pool_state_slot_base = U256::from_be_slice(pool_state_slot.as_slice());

    let liquidity_slot = pool_state_slot_base + U256::from(POOL_MANAGER_POOL_LIQUIDITY_SLOT_OFFSET);

    let liquidity = slot_fetcher
        .storage_at(pool_manager_address, liquidity_slot.into(), block_number)
        .await?;

    Ok(liquidity)
}

pub async fn pool_manager_load_tick_map<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    tick_spacing: I24,
    start_tick: Option<I24>,
    end_tick: Option<I24>,
    load_buffer: Option<usize>,
    skip_uninitialized: bool
) -> eyre::Result<HashMap<I24, TickData>> {
    let start_tick = start_tick
        .map(|t| normalize_tick(t, tick_spacing))
        .unwrap_or(min_valid_tick(tick_spacing));
    let end_tick = end_tick
        .map(|t| normalize_tick(t, tick_spacing))
        .unwrap_or(max_valid_tick(tick_spacing));

    let mut tick_data_loading_stream = futures::stream::iter(
        (start_tick.as_i32()..=end_tick.as_i32()).step_by(tick_spacing.as_i32().abs() as usize)
    )
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
    .buffer_unordered(load_buffer.unwrap_or(1000));

    let num_to_load = ((end_tick - start_tick) / tick_spacing).as_i32();
    let mut i = 0;
    let mut loaded_tick_data = HashMap::new();
    println!("starting tick loading");
    while let Some(val) = tick_data_loading_stream.next().await {
        let (k, v) = val?;
        if !skip_uninitialized || v.is_initialized {
            loaded_tick_data.insert(k, v);
        }
        i += 1;
        if i % 100 == 0 || i - 1 == num_to_load {
            println!("LOADED: {i}/{num_to_load}");
        }
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

pub async fn tick_initialized<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    tick_spacing: I24,
    pool_id: PoolId,
    tick: I24
) -> eyre::Result<bool> {
    let (word_pos, bit_pos) = tick_position_from_compressed(tick, tick_spacing);
    let pool_tick_bitmap_slot = pool_manager_pool_tick_bitmap_slot(pool_id.into(), word_pos);

    let is_initialized_value = slot_fetcher
        .storage_at(pool_manager_address, pool_tick_bitmap_slot.into(), block_number)
        .await?;

    Ok(is_initialized_value & (U256::ONE << U256::from(bit_pos)) != U256::ZERO)
}

#[cfg(test)]
mod tests {

    use alloy_primitives::{U160, aliases::U24};
    use angstrom_types::{self, primitive::POOL_MANAGER_ADDRESS};

    use super::*;
    use crate::{
        apis::AngstromDataApi,
        test_utils::valid_test_params::init_valid_position_params_with_provider
    };

    #[tokio::test]
    async fn test_pool_manager_position_state_liquidity() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = pool_manager_position_state_liquidity(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into(),
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        assert_eq!(results, pos_info.position_liquidity);
    }

    #[tokio::test]
    async fn test_pool_manager_pool_state_last_fee_growth_global() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = pool_manager_pool_last_fee_growth_global(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into()
        )
        .await
        .unwrap();

        println!("{results:?}");

        // assert_eq!(results, pos_info.position_liquidity);
    }

    #[tokio::test]
    async fn test_pool_manager_position_state_last_fee_growth_inside() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let results = pool_manager_position_state_last_fee_growth_inside(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into(),
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        println!("{results:?}");

        // assert_eq!(results, pos_info.position_liquidity);
    }

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
    async fn test_pool_manager_position_fee_growth_inside() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let results = pool_manager_position_fee_growth_inside(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into(),
            pos_info.current_pool_tick,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        println!("{results:?}");

        // assert_eq!(results, pos_info.position_liquidity);
    }

    #[tokio::test]
    async fn test_pool_manager_pool_slot0() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let results = pool_manager_pool_slot0(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into()
        )
        .await
        .unwrap();

        let expected = UnpackedSlot0 {
            sqrt_price_x96: U160::from(1081670548984259501374925403766425_u128),
            tick:           I24::unchecked_from(190443),
            protocol_fee:   U24::ZERO,
            lp_fee:         U24::ZERO
        };

        assert_eq!(results, expected);
    }

    #[tokio::test]
    async fn test_pool_manager_pool_liquidity() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let results = pool_manager_pool_liquidity(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into()
        )
        .await
        .unwrap();

        assert_eq!(results, U256::from(435906614777942732_u128));
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
            None,
            None,
            true
        )
        .await
        .unwrap();

        assert_eq!(results.len(), 8);
    }
}
