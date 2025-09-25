use alloy_primitives::{
    Address, I64, U256,
    aliases::{I24, U24}
};
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

pub fn tick_position_from_compressed(tick: I24, tick_spacing: I24) -> (i16, u8) {
    let compressed = compress_tick(tick, tick_spacing);
    _tick_position_from_compressed(compressed)
}

pub fn tick_position_from_compressed_inequality(
    tick: I24,
    tick_spacing: I24,
    add_sub: I24
) -> (i16, u8) {
    let compressed = compress_tick(tick, tick_spacing) + add_sub;
    _tick_position_from_compressed(compressed)
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

fn _tick_position_from_compressed(compressed: I24) -> (i16, u8) {
    let compressed_i32 = compressed.as_i32();
    let word_pos = (compressed_i32 >> 8) as i16;
    let bit_pos = (compressed_i32 & 0xff) as u8;

    (word_pos, bit_pos)
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
            255u16 - word_shifted.leading_zeros() as u16
        };

        let initialized = relative_pos != 256;
        let next_bit_pos =
            if initialized { (relative_pos as u8).saturating_sub(offset) } else { 0u8 };

        (initialized, next_bit_pos)
    }
}

pub async fn next_initialized_tick_within_one_word<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    tick: I24,
    tick_spacing: I24,
    lte: bool
) -> eyre::Result<(I24, bool)> {
    /*

           int24 compressed = compress(tick, tickSpacing);
           if (lte) {
               (int16 wordPos, uint8 bitPos) = position(compressed);
               // all the 1s at or to the right of the current bitPos
               uint256 mask = type(uint256).max >> (uint256(type(uint8).max) - bitPos);
               uint256 masked = self[wordPos] & mask;

               // if there are no initialized ticks to the right of or at the current tick, return rightmost in the word
               initialized = masked != 0;
               // overflow/underflow is possible, but prevented externally by limiting both tickSpacing and tick
               next = initialized
                   ? (compressed - int24(uint24(bitPos - BitMath.mostSignificantBit(masked)))) * tickSpacing
                   : (compressed - int24(uint24(bitPos))) * tickSpacing;
           } else {
               // start from the word of the next tick, since the current tick state doesn't matter
               (int16 wordPos, uint8 bitPos) = position(++compressed);
               // all the 1s at or to the left of the bitPos
               uint256 mask = ~((1 << bitPos) - 1);
               uint256 masked = self[wordPos] & mask;

               // if there are no initialized ticks to the left of the current tick, return leftmost in the word
               initialized = masked != 0;
               // overflow/underflow is possible, but prevented externally by limiting both tickSpacing and tick
               next = initialized
                   ? (compressed + int24(uint24(BitMath.leastSignificantBit(masked) - bitPos))) * tickSpacing
                   : (compressed + int24(uint24(type(uint8).max - bitPos))) * tickSpacing;
           }

    */

    let compressed = compress_tick(tick, tick_spacing);
    if lte {
        let (word_pos, bit_pos) = _tick_position_from_compressed(compressed);
        let mask = U256::MAX >> (U256::from(u8::MAX) - U256::from(bit_pos));
        let masked = tick_bitmap_from_word(
            slot_fetcher,
            pool_manager_address,
            block_number,
            pool_id,
            word_pos
        )
        .await?
        .0 & mask;

        let initialized = masked != U256::ZERO;
        let next = if initialized {
            (compressed - I24::unchecked_from(bit_pos - most_significant_bit(masked)))
                * tick_spacing
        } else {
            (compressed - I24::unchecked_from(bit_pos)) * tick_spacing
        };
        Ok((next, initialized))
    } else {
        let (word_pos, bit_pos) = _tick_position_from_compressed(compressed + I24::ONE);
        let mask = U256::from(!((1 << bit_pos) - 1));
        let masked = tick_bitmap_from_word(
            slot_fetcher,
            pool_manager_address,
            block_number,
            pool_id,
            word_pos
        )
        .await?
        .0 & mask;

        let initialized = masked != U256::ZERO;
        let next = if initialized {
            (compressed + I24::unchecked_from(least_significant_bit(masked) - bit_pos))
                * tick_spacing
        } else {
            (compressed + I24::unchecked_from(U24::from(u8::MAX - bit_pos))) * tick_spacing
        };
        Ok((next, initialized))
    }
}

fn most_significant_bit(x: U256) -> u8 {
    /*

        /// @notice Returns the index of the most significant bit of the number,
        ///     where the least significant bit is at index 0 and the most significant bit is at index 255
        /// @param x the value for which to compute the most significant bit, must be greater than 0
        /// @return r the index of the most significant bit
        function mostSignificantBit(uint256 x) internal pure returns (uint8 r) {
            require(x > 0);

            assembly ("memory-safe") {
                r := shl(7, lt(0xffffffffffffffffffffffffffffffff, x))
                r := or(r, shl(6, lt(0xffffffffffffffff, shr(r, x))))
                r := or(r, shl(5, lt(0xffffffff, shr(r, x))))
                r := or(r, shl(4, lt(0xffff, shr(r, x))))
                r := or(r, shl(3, lt(0xff, shr(r, x))))
                // forgefmt: disable-next-item
                r := or(r, byte(and(0x1f, shr(shr(r, x), 0x8421084210842108cc6318c6db6d54be)),
                    0x0706060506020500060203020504000106050205030304010505030400000000))
            }
        }
    */
}

fn least_significant_bit(x: U256) -> u8 {
    /*

        /// @notice Returns the index of the least significant bit of the number,
        ///     where the least significant bit is at index 0 and the most significant bit is at index 255
        /// @param x the value for which to compute the least significant bit, must be greater than 0
        /// @return r the index of the least significant bit
        function leastSignificantBit(uint256 x) internal pure returns (uint8 r) {
            require(x > 0);

            assembly ("memory-safe") {
                // Isolate the least significant bit.
                x := and(x, sub(0, x))
                // For the upper 3 bits of the result, use a De Bruijn-like lookup.
                // Credit to adhusson: https://blog.adhusson.com/cheap-find-first-set-evm/
                // forgefmt: disable-next-item
                r := shl(5, shr(252, shl(shl(2, shr(250, mul(x,
                    0xb6db6db6ddddddddd34d34d349249249210842108c6318c639ce739cffffffff))),
                    0x8040405543005266443200005020610674053026020000107506200176117077)))
                // For the lower 5 bits of the result, use a De Bruijn lookup.
                // forgefmt: disable-next-item
                r := or(r, byte(and(div(0xd76453e0, shr(r, x)), 0x1f),
                    0x001f0d1e100c1d070f090b19131c1706010e11080a1a141802121b1503160405))
            }
        }
    */
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
        .storage_at(pool_manager_address, pool_tick_bitmap_slot, block_number)
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

pub async fn next_tick_gt<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    tick_spacing: I24,
    pool_id: PoolId,
    tick: I24,
    initialized_only: bool
) -> eyre::Result<(bool, I24)> {
    if is_tick_at_bounds(tick, tick_spacing, false) {
        return Ok((false, tick));
    }

    let (word_pos, bit_pos) =
        tick_position_from_compressed_inequality(tick, tick_spacing, I24::unchecked_from(1));
    let tick_bitmap =
        tick_bitmap_from_word(slot_fetcher, pool_manager_address, block_number, pool_id, word_pos)
            .await?;

    let (is_initialized, next_bit_pos) = tick_bitmap.next_bit_pos_gte(bit_pos);
    let next_tick = tick_from_word_and_bit_pos(word_pos, next_bit_pos, tick_spacing);
    if !initialized_only
        || is_initialized
        || I24::unchecked_from(MAX_TICK) - next_tick <= tick_spacing
    {
        Ok((is_initialized, next_tick))
    } else {
        Box::pin(next_tick_gt(
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

pub async fn next_tick_lt<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    tick_spacing: I24,
    pool_id: PoolId,
    tick: I24,
    initialized_only: bool
) -> eyre::Result<(bool, I24)> {
    if is_tick_at_bounds(tick, tick_spacing, true) {
        return Ok((false, tick));
    }

    let (word_pos, bit_pos) =
        tick_position_from_compressed_inequality(tick, tick_spacing, I24::unchecked_from(-1));
    let tick_bitmap =
        tick_bitmap_from_word(slot_fetcher, pool_manager_address, block_number, pool_id, word_pos)
            .await?;

    let (is_initialized, next_bit_pos) = tick_bitmap.next_bit_pos_lte(bit_pos);
    let next_tick = tick_from_word_and_bit_pos(word_pos, next_bit_pos, tick_spacing);
    if !initialized_only
        || is_initialized
        || next_tick - I24::unchecked_from(MIN_TICK) <= tick_spacing
    {
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

pub async fn next_tick_le<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    tick_spacing: I24,
    pool_id: PoolId,
    tick: I24,
    initialized_only: bool
) -> eyre::Result<(bool, I24)> {
    if is_tick_at_bounds(tick, tick_spacing, true) {
        return Ok((false, tick));
    }

    let (word_pos, bit_pos) = tick_position_from_compressed(tick, tick_spacing);
    let tick_bitmap =
        tick_bitmap_from_word(slot_fetcher, pool_manager_address, block_number, pool_id, word_pos)
            .await?;

    let (is_initialized, next_bit_pos) = tick_bitmap.next_bit_pos_lte(bit_pos);
    let next_tick = tick_from_word_and_bit_pos(word_pos, next_bit_pos, tick_spacing);
    if !initialized_only
        || is_initialized
        || next_tick - I24::unchecked_from(MIN_TICK) <= tick_spacing
    {
        Ok((is_initialized, next_tick))
    } else {
        Box::pin(next_tick_le(
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

fn is_tick_at_bounds(tick: I24, tick_spacing: I24, is_decreasing: bool) -> bool {
    let tick = I64::from(tick);
    let tick_spacing = I64::from(tick_spacing);
    let min = I64::unchecked_from(MIN_TICK);
    let max = I64::unchecked_from(MAX_TICK);

    if is_decreasing { tick - tick_spacing.abs() <= min } else { tick + tick_spacing.abs() >= max }
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
    async fn test_next_tick_gt() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;
        let tick = I24::unchecked_from(190990);
        let tick_spacing = pos_info.pool_key.tickSpacing;
        let pool_id = pos_info.pool_key.into();

        let (_, results) = next_tick_gt(
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
        let tick = I24::unchecked_from(192311);
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

        assert_eq!(results, I24::unchecked_from(191130));
    }

    #[tokio::test]
    async fn test_next_tick_le() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;
        let tick = I24::unchecked_from(192311);
        let tick_spacing = pos_info.pool_key.tickSpacing;
        let pool_id = pos_info.pool_key.into();

        let (_, results) = next_tick_le(
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

        assert_eq!(results, I24::unchecked_from(192310));
    }
}
