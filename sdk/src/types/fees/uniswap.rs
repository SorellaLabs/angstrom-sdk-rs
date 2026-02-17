use alloy_eips::BlockId;
use alloy_primitives::{Address, U256, aliases::I24};
use angstrom_types_primitives::primitive::PoolId;
use uniswap_storage::{
    self, StorageSlotFetcher,
    v4::pool_manager::position_state::{
        pool_manager_position_fee_growth_inside, pool_manager_position_state_last_fee_growth_inside
    }
};

pub async fn uniswap_fee_deltas<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    position_manager_address: Address,
    block_id: BlockId,
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
            pool_id,
            current_pool_tick,
            tick_lower,
            tick_upper,
            block_id,
        ),
        pool_manager_position_state_last_fee_growth_inside(
            slot_fetcher,
            pool_manager_address,
            position_manager_address,
            pool_id,
            position_token_id,
            tick_lower,
            tick_upper,
            block_id,
        ),
    )?;

    Ok((
        fee_growth_inside0_x128 - fee_growth_inside0_last_x128,
        fee_growth_inside1_x128 - fee_growth_inside1_last_x128
    ))
}

#[cfg(test)]
mod tests {
    use angstrom_types_primitives::primitive::{POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS};

    use super::*;

    #[cfg(feature = "l1")]
    #[tokio::test]
    async fn test_uniswap_fee_deltas_l1() {
        use crate::l1::test_utils::valid_test_params::init_valid_position_params_with_provider;
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        #[cfg(feature = "local-reth")]
        let provider = &provider.provider_ref().eth_api();
        #[cfg(not(feature = "local-reth"))]
        let provider = &provider;

        let results = uniswap_fee_deltas(
            provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            block_number.into(),
            pos_info.pool_id,
            pos_info.current_pool_tick,
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        assert_eq!(
            results,
            (
                U256::from(4004676340914304001936429601015_u128),
                U256::from_str_radix("1565824208245443875813344119471164423504", 10).unwrap()
            )
        );
    }

    #[cfg(feature = "l2")]
    #[tokio::test]
    async fn test_uniswap_fee_deltas_l2() {
        use crate::l2::test_utils::valid_test_params::init_valid_position_params_with_provider;
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        #[cfg(feature = "local-reth")]
        let provider = &provider.provider_ref().eth_api();
        #[cfg(not(feature = "local-reth"))]
        let provider = &provider;

        let results = uniswap_fee_deltas(
            provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            block_number.into(),
            pos_info.pool_id,
            pos_info.current_pool_tick,
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        assert_eq!(
            results,
            (
                U256::from(4004676340914304001936429601015_u128),
                U256::from_str_radix("1565824208245443875813344119471164423504", 10).unwrap()
            )
        );
    }
}
