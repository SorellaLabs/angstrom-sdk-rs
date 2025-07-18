use alloy_primitives::{B256, I64, U256, U512, aliases::I24, b256, keccak256};
use angstrom_types::primitive::POSITION_MANAGER_ADDRESS;

pub const FIXED_POINT_128: B256 =
    b256!("0x0000000000000000000000000000000100000000000000000000000000000000");

pub const MIN_TICK: i32 = -887272;
pub const MAX_TICK: i32 = 887272;

pub fn encode_position_key(position_token_id: U256, tick_lower: I24, tick_upper: I24) -> B256 {
    let mut bytes = [0u8; 70];
    bytes[12..32].copy_from_slice(&***POSITION_MANAGER_ADDRESS.get().unwrap());
    bytes[32..35].copy_from_slice(&tick_lower.to_be_bytes::<3>());
    bytes[35..38].copy_from_slice(&tick_upper.to_be_bytes::<3>());
    bytes[38..].copy_from_slice(&*B256::from(position_token_id));
    keccak256(&bytes[12..])
}

pub fn full_mul_x128(x: U256, y: U256) -> U256 {
    if x.is_zero() || y.is_zero() {
        return U256::ZERO;
    }

    let prod: U512 = U512::from(x) * U512::from(y);

    let shifted: U512 = prod >> 128u32;

    if (shifted >> 256u32) != U512::ZERO {
        panic!("We check the final result doesn't overflow by checking that p1_0 = 0"); // same condition that triggers revert in Solidity
    }

    U256::from(shifted)
}

pub fn mul_div(a: U256, b: U256, denominator: U256) -> U256 {
    if denominator.is_zero() {
        panic!("require(denominator != 0)");
    }

    // 512-bit product
    let product: U512 = U512::from(a) * U512::from(b);

    // Split into high / low 256-bit words
    let mask_256: U512 = U512::from(U256::MAX); // 2^256 − 1
    let prod0 = U256::from(product & mask_256); // low 256 bits
    let prod1 = U256::from(product >> 256u32); // high 256 bits

    // Overflow check (denominator must be > prod1)
    if denominator <= prod1 {
        panic!("require(denominator > prod1)");
    }

    if prod1.is_zero() {
        return prod0 / denominator;
    }

    let quotient = product / U512::from(denominator);
    U256::from(quotient)
}

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

pub fn max_valid_tick(tick_spacing: I24) -> I24 {
    I24::unchecked_from(MAX_TICK) / tick_spacing * tick_spacing
}

pub fn min_valid_tick(tick_spacing: I24) -> I24 {
    I24::unchecked_from(MIN_TICK) / tick_spacing * tick_spacing
}

#[cfg(test)]
mod math_tests {
    use super::*;
    #[test]
    fn test_full_mul_x128() {}

    #[test]
    fn test_mul_div() {
        let mult = U256::from_str_radix("587456364760337352996937067840847760644036", 10).unwrap();
        let liq = U256::from(6047841786519_u128);

        let initial = mult / liq;

        let mul_div = mul_div(initial, liq, U256::from_be_slice(FIXED_POINT_128.as_slice()));
        println!("{mul_div:?}");
    }
}
