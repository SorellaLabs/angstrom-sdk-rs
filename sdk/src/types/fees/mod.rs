mod uniswap;
use alloy_primitives::U256;
pub use uniswap::*;
use uniswap_storage::v4::utils::{FIXED_POINT_128, full_mul_x128, mul_div};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LiquidityPositionFees {
    pub position_liquidity:   u128,
    /// l1 -> token0
    /// l2 -> token0 (native ETH)
    pub angstrom_token0_fees: U256,
    pub uniswap_token0_fees:  U256,
    pub uniswap_token1_fees:  U256
}

impl LiquidityPositionFees {
    pub fn new(
        position_liquidity: u128,
        angstrom_fee_delta: U256,
        uniswap_token0_fee_delta: U256,
        uniswap_token1_fee_delta: U256
    ) -> Self {
        let pl = U256::from(position_liquidity);
        Self {
            position_liquidity,
            angstrom_token0_fees: full_mul_x128(angstrom_fee_delta, pl),
            uniswap_token0_fees: mul_div(uniswap_token0_fee_delta, pl, FIXED_POINT_128.into()),
            uniswap_token1_fees: mul_div(uniswap_token1_fee_delta, pl, FIXED_POINT_128.into())
        }
    }
}
