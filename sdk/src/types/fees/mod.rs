mod angstrom;
use alloy_primitives::{Address, U256, aliases::I24};
pub use angstrom::*;
mod uniswap;
use angstrom_types_primitives::primitive::PoolId;
pub use uniswap::*;
use uniswap_storage::{
    StorageSlotFetcher,
    v4::utils::{FIXED_POINT_128, full_mul_x128, mul_div}
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LiquidityPositionFees {
    pub position_liquidity:   u128,
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

pub async fn position_fees<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    angstrom_address: Address,
    position_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    current_pool_tick: I24,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24,
    position_liquidity: u128
) -> eyre::Result<LiquidityPositionFees> {
    let (angstrom_fee_delta, (uniswap_token0_fee_delta, uniswap_token1_fee_delta)) = tokio::try_join!(
        angstrom_fee_delta_x128(
            slot_fetcher,
            angstrom_address,
            position_manager_address,
            block_number,
            pool_id,
            current_pool_tick,
            position_token_id,
            tick_lower,
            tick_upper
        ),
        uniswap_fee_deltas(
            slot_fetcher,
            pool_manager_address,
            position_manager_address,
            block_number,
            pool_id,
            current_pool_tick,
            position_token_id,
            tick_lower,
            tick_upper
        )
    )?;

    Ok(LiquidityPositionFees::new(
        position_liquidity,
        angstrom_fee_delta,
        uniswap_token0_fee_delta,
        uniswap_token1_fee_delta
    ))
}

#[cfg(test)]
mod tests {
    use angstrom_types_primitives::primitive::{
        ANGSTROM_ADDRESS, POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS
    };

    use super::*;
    use crate::test_utils::valid_test_params::init_valid_position_params_with_provider;

    #[tokio::test]
    async fn test_position_fees() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        #[cfg(feature = "local-reth")]
        let provider = provider.provider_ref();
        #[cfg(not(feature = "local-reth"))]
        let provider = &provider;

        let results = position_fees(
            provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            *ANGSTROM_ADDRESS.get().unwrap(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_id,
            pos_info.current_pool_tick,
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper,
            pos_info.position_liquidity
        )
        .await
        .unwrap();

        assert_eq!(
            results,
            LiquidityPositionFees {
                position_liquidity:   807449445327074,
                angstrom_token0_fees: U256::from(214093138_u128),
                uniswap_token0_fees:  U256::from(9502619_u128),
                uniswap_token1_fees:  U256::from(3715513971140452_u128)
            }
        );
    }
}
