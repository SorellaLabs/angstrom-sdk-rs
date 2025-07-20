use alloy_primitives::{Address, B256, U256, keccak256};
use alloy_sol_types::SolValue;
use angstrom_types::primitive::PoolId;

use crate::types::{
    StorageSlotFetcher,
    contracts::{UnpackSlot0, UnpackedSlot0}
};

// pool state
pub const POOL_MANAGER_POOL_STATE_MAP_SLOT: u8 = 6;
pub const POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL0_X128_SLOT_OFFSET: u8 = 1;
pub const POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL1_X128_SLOT_OFFSET: u8 = 2;
pub const POOL_MANAGER_POOL_LIQUIDITY_SLOT_OFFSET: u8 = 3;

pub fn pool_manager_pool_state_slot(pool_id: U256) -> B256 {
    keccak256((pool_id, U256::from(POOL_MANAGER_POOL_STATE_MAP_SLOT)).abi_encode())
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

#[cfg(test)]
mod tests {

    use alloy_primitives::{
        U160,
        aliases::{I24, U24}
    };
    use angstrom_types::{self, primitive::POOL_MANAGER_ADDRESS};

    use super::*;
    use crate::{
        test_utils::valid_test_params::init_valid_position_params_with_provider,
        types::contracts::UnpackedSlot0
    };

    #[tokio::test]
    async fn test_pool_manager_pool_last_fee_growth_global() {
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
}
