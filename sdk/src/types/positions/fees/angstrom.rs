use alloy_primitives::{Address, B256, U256, aliases::I24};
use angstrom_types::primitive::PoolId;

use crate::types::{
    StorageSlotFetcher,
    positions::{angstrom_growth_inside, angstrom_last_growth_inside}
};

pub async fn angstrom_position_rewards<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    tick_lower: I24,
    tick_upper: I24,
    current_tick: I24,
    position_liquidity: U256,
    position_key: B256
) -> eyre::Result<U256> {
    let (growth_inside, last_growth_inside) = tokio::try_join!(
        angstrom_growth_inside(
            slot_fetcher,
            angstrom_address,
            block_number,
            pool_id,
            tick_lower,
            tick_upper,
            current_tick
        ),
        angstrom_last_growth_inside(
            slot_fetcher,
            angstrom_address,
            block_number,
            pool_id,
            position_key
        )
    )?;

    Ok((growth_inside - last_growth_inside) * position_liquidity)
}

#[cfg(test)]
mod tests {
    use angstrom_types::primitive::{ANGSTROM_ADDRESS, POOL_MANAGER_ADDRESS};

    use super::*;
    use crate::{
        apis::AngstromDataApi,
        test_utils::valid_test_params::init_valid_position_params_with_provider,
        types::positions::pool_manager_liquidity
    };

    #[tokio::test]
    async fn test_angstrom_position_rewards() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let position_liquidity = pool_manager_liquidity(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key,
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        let (_, pool) = provider
            .pool_data_by_pool_id(pos_info.pool_id, Some(block_number))
            .await
            .unwrap();

        let results = angstrom_position_rewards(
            &provider,
            *ANGSTROM_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_id,
            pos_info.tick_lower,
            pos_info.tick_upper,
            I24::unchecked_from(pool.tick),
            U256::from(position_liquidity),
            pos_info.angstrom_rewards_position_key
        )
        .await
        .unwrap();

        assert_eq!(
            results,
            U256::from_str_radix("120172277127583782077734552915892808915697", 10).unwrap()
        );
    }
}
