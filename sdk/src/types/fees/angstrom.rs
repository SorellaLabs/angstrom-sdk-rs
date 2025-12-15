use alloy_primitives::{Address, U256, aliases::I24};
use angstrom_types_primitives::primitive::PoolId;
use uniswap_storage::{
    StorageSlotFetcher,
    angstrom::mainnet::{angstrom_growth_inside, angstrom_last_growth_inside} /* angstrom::{angstrom_growth_inside, angstrom_last_growth_inside} */
};

pub async fn angstrom_fee_delta_x128<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    position_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    current_pool_tick: I24,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<U256> {
    let (growth_inside, last_growth_inside) = tokio::try_join!(
        angstrom_growth_inside(
            slot_fetcher,
            angstrom_address,
            pool_id,
            current_pool_tick,
            tick_lower,
            tick_upper,
            block_number,
        ),
        angstrom_last_growth_inside(
            slot_fetcher,
            angstrom_address,
            position_manager_address,
            pool_id,
            position_token_id,
            tick_lower,
            tick_upper,
            block_number,
        ),
    )?;

    Ok(growth_inside - last_growth_inside)
}

#[cfg(test)]
mod tests {
    use angstrom_types_primitives::primitive::{ANGSTROM_ADDRESS, POSITION_MANAGER_ADDRESS};

    use super::*;
    use crate::test_utils::valid_test_params::init_valid_position_params_with_provider;

    #[tokio::test]
    async fn test_angstrom_fee_delta_x128() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        #[cfg(feature = "local-reth")]
        let provider = provider.provider_ref();
        #[cfg(not(feature = "local-reth"))]
        let provider = &provider;

        let results = angstrom_fee_delta_x128(
            provider,
            *ANGSTROM_ADDRESS.get().unwrap(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_id,
            pos_info.current_pool_tick,
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        let expected = U256::from(90224992210989852552811100631246_u128);
        assert_eq!(results, expected);
    }
}
