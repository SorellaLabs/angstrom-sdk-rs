use alloy_primitives::{Address, I64, U256, aliases::I24};
use angstrom_types::primitive::PoolId;
use serde::{Deserialize, Serialize};

use crate::types::{
    StorageSlotFetcher,
    contracts::{
        pool_manager::pool_tick_state::pool_manager_pool_tick_bitmap_slot,
        utils::{MAX_TICK, MIN_TICK}
    }
};

pub fn compress_tick(tick: I24, tick_spacing: I24) -> I24 {
    tick.saturating_div(tick_spacing)
        - if tick % tick_spacing < I24::ZERO { I24::ONE } else { I24::ZERO }
}

pub fn tick_position_from_compressed(mut tick: I24, tick_spacing: I24) -> (i16, u8) {
    if tick % tick_spacing != I24::ZERO {
        tick = normalize_tick(tick, tick_spacing);
    }

    let compressed = compress_tick(tick, tick_spacing);

    try_tick_position_from_compressed(compressed).unwrap()
}

pub fn tick_position_from_compressed_inequality(
    mut tick: I24,
    tick_spacing: I24,
    add_sub: I24
) -> (i16, u8) {
    if tick % tick_spacing != I24::ZERO {
        tick = normalize_tick(tick, tick_spacing);
    }

    let compressed = compress_tick(tick, tick_spacing) + add_sub;

    try_tick_position_from_compressed(compressed).unwrap()
}

pub fn normalize_tick(tick: I24, tick_spacing: I24) -> I24 {
    let norm = compress_tick(tick, tick_spacing) * tick_spacing;

    if I64::from(tick) > I64::from(norm) + I64::from(tick_spacing)
        || I64::from(tick) < I64::from(norm) - I64::from(tick_spacing)
        || norm.as_i32() < MIN_TICK
        || norm.as_i32() > MAX_TICK
    {
        if tick.is_negative() {
            return normalize_tick(tick + tick_spacing.abs(), tick_spacing);
        } else {
            return normalize_tick(tick - tick_spacing.abs(), tick_spacing);
        }
    }

    norm
}

fn try_tick_position_from_compressed(compressed: I24) -> Option<(i16, u8)> {
    let compressed_i32 = compressed.as_i32();
    let word_pos = (compressed_i32 >> 8) as i16;
    let bit_pos = (compressed_i32 & 0xff) as u8;

    Some((word_pos, bit_pos))
}

pub fn tick_from_word_and_bit_pos(word_pos: i16, bit_pos: u8, tick_spacing: I24) -> I24 {
    (I24::unchecked_from(word_pos) * I24::unchecked_from(256) + I24::unchecked_from(bit_pos))
        * tick_spacing
}

#[derive(Debug, Clone, Copy, Hash, Serialize, Deserialize)]
pub struct TickBitmap(pub U256);

impl TickBitmap {
    pub fn is_initialized(&self, bit_pos: u8) -> bool {
        self.0 & (U256::ONE << U256::from(bit_pos)) != U256::ZERO
    }

    pub fn next_bit_pos_gte(&self, bit_pos: u8) -> (bool, u8) {
        let word_shifted = self.0 >> U256::from(bit_pos);

        let relative_pos =
            if word_shifted == U256::ZERO { 256u16 } else { word_shifted.trailing_zeros() as u16 };

        let initialized = relative_pos != 256;
        let next_bit_pos = if initialized { (relative_pos as u8) + bit_pos } else { u8::MAX };

        (initialized, next_bit_pos)
    }

    pub fn next_bit_pos_lte(&self, bit_pos: u8) -> (bool, u8) {
        let offset = 0xff - bit_pos;

        let word_shifted = self.0 << U256::from(offset);

        let relative_pos = if word_shifted == U256::ZERO {
            256u16
        } else {
            256u16 - word_shifted.leading_zeros() as u16
        };

        let initialized = relative_pos != 256;
        let next_bit_pos =
            if initialized { (relative_pos as u8).saturating_sub(offset) } else { 0u8 };

        (initialized, next_bit_pos)
    }
}

pub async fn tick_bitmap_from_word<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    word_pos: i16
) -> eyre::Result<TickBitmap> {
    let pool_tick_bitmap_slot = pool_manager_pool_tick_bitmap_slot(pool_id.into(), word_pos);

    let tick_bitmap = slot_fetcher
        .storage_at(pool_manager_address, pool_tick_bitmap_slot.into(), block_number)
        .await?;

    Ok(TickBitmap(tick_bitmap))
}

pub async fn tick_bitmap_from_tick<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    tick: I24,
    tick_spacing: I24
) -> eyre::Result<TickBitmap> {
    let (word_pos, _) = tick_position_from_compressed(tick, tick_spacing);

    tick_bitmap_from_word(slot_fetcher, pool_manager_address, block_number, pool_id, word_pos).await
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
    let tick_bitmap =
        tick_bitmap_from_word(slot_fetcher, pool_manager_address, block_number, pool_id, word_pos)
            .await?;

    Ok(tick_bitmap.is_initialized(bit_pos))
}

pub async fn next_tick_ge<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    tick_spacing: I24,
    pool_id: PoolId,
    tick: I24,
    initialized_only: bool
) -> eyre::Result<(bool, I24)> {
    let (word_pos, bit_pos) =
        tick_position_from_compressed_inequality(tick, tick_spacing, I24::unchecked_from(1));
    let tick_bitmap =
        tick_bitmap_from_word(slot_fetcher, pool_manager_address, block_number, pool_id, word_pos)
            .await?;

    let (is_initialized, next_bit_pos) = tick_bitmap.next_bit_pos_gte(bit_pos);
    let next_tick = tick_from_word_and_bit_pos(word_pos, next_bit_pos, tick_spacing);
    if !initialized_only || is_initialized {
        Ok((is_initialized, next_tick))
    } else {
        Box::pin(next_tick_ge(
            slot_fetcher,
            pool_manager_address,
            block_number,
            tick_spacing,
            pool_id,
            tick,
            initialized_only
        ))
        .await
    }
}

pub async fn next_tick_lt<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    tick_spacing: I24,
    pool_id: PoolId,
    tick: I24,
    initialized_only: bool
) -> eyre::Result<(bool, I24)> {
    let (word_pos, bit_pos) =
        tick_position_from_compressed_inequality(tick, tick_spacing, I24::unchecked_from(-1));
    let tick_bitmap =
        tick_bitmap_from_word(slot_fetcher, pool_manager_address, block_number, pool_id, word_pos)
            .await?;

    let (is_initialized, next_bit_pos) = tick_bitmap.next_bit_pos_lte(bit_pos);
    let next_tick = tick_from_word_and_bit_pos(word_pos, next_bit_pos, tick_spacing);
    if !initialized_only || is_initialized {
        Ok((is_initialized, next_tick))
    } else {
        Box::pin(next_tick_lt(
            slot_fetcher,
            pool_manager_address,
            block_number,
            tick_spacing,
            pool_id,
            next_tick,
            initialized_only
        ))
        .await
    }
}

#[cfg(test)]
mod tests {
    use angstrom_types::{self, primitive::POOL_MANAGER_ADDRESS};

    use super::*;
    use crate::test_utils::valid_test_params::init_valid_position_params_with_provider;

    #[tokio::test]
    async fn test_tick_bitmap() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;
        let pool_id = pos_info.pool_key.into();

        let results = tick_bitmap_from_word(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pool_id,
            346
        )
        .await
        .unwrap();
        assert_eq!(
            results.0,
            U256::from_str_radix("2854495385411919762116571938898990272765493248", 10).unwrap()
        );
    }

    #[tokio::test]
    async fn test_tick_initialized() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;
        let tick = I24::unchecked_from(190990);
        let tick_spacing = pos_info.pool_key.tickSpacing;
        let pool_id = pos_info.pool_key.into();

        let results = tick_initialized(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            tick_spacing,
            pool_id,
            tick
        )
        .await
        .unwrap();
        assert!(results);
    }

    #[tokio::test]
    async fn test_next_tick_ge() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;
        let tick = I24::unchecked_from(190990);
        let tick_spacing = pos_info.pool_key.tickSpacing;
        let pool_id = pos_info.pool_key.into();

        let (_, results) = next_tick_ge(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            tick_spacing,
            pool_id,
            tick,
            true
        )
        .await
        .unwrap();

        assert_eq!(results, I24::unchecked_from(191120));
    }

    #[tokio::test]
    async fn test_next_tick_lt() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;
        let tick = I24::unchecked_from(190990);
        let tick_spacing = pos_info.pool_key.tickSpacing;
        let pool_id = pos_info.pool_key.into();

        let (_, results) = next_tick_lt(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            tick_spacing,
            pool_id,
            tick,
            true
        )
        .await
        .unwrap();

        assert_eq!(results, I24::unchecked_from(189130));
    }
}
