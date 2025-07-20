use alloy_primitives::{Address, U256, aliases::I24};
use angstrom_types::primitive::PoolId;

use crate::types::{
    StorageSlotFetcher,
    contracts::pool_manager::position_state::{
        pool_manager_position_fee_growth_inside, pool_manager_position_state_last_fee_growth_inside
    }
};

pub async fn uniswap_fee_deltas<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    current_pool_tick: I24,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<(U256, U256)> {
    let (
        (fee_growth_inside0_x128, fee_growth_inside1_x128),
        (fee_growth_inside0_last_x128, fee_growth_inside1_last_x128)
    ) = tokio::try_join!(
        pool_manager_position_fee_growth_inside(
            slot_fetcher,
            pool_manager_address,
            block_number,
            pool_id,
            current_pool_tick,
            tick_lower,
            tick_upper,
        ),
        pool_manager_position_state_last_fee_growth_inside(
            slot_fetcher,
            pool_manager_address,
            block_number,
            pool_id,
            position_token_id,
            tick_lower,
            tick_upper
        ),
    )?;

    Ok((
        fee_growth_inside0_x128 - fee_growth_inside0_last_x128,
        fee_growth_inside1_x128 - fee_growth_inside1_last_x128
    ))
}

#[cfg(test)]
mod tests {
    use angstrom_types::primitive::POOL_MANAGER_ADDRESS;

    use super::*;
    use crate::test_utils::valid_test_params::init_valid_position_params_with_provider;

    #[tokio::test]
    async fn test_uniswap_fee_deltas() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = uniswap_fee_deltas(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_id,
            pos_info.current_pool_tick,
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        println!("{results:?}");

        // assert_eq!(
        //     results,
        //     U256::from_str_radix("120172277127583782077734552915892808915697"
        // , 10).unwrap() );
    }
}
