use alloy_primitives::{Address, B256, U256, aliases::I24, keccak256};
use alloy_sol_types::SolValue;
use angstrom_types::primitive::PoolId;

use crate::types::{
    StorageSlotFetcher,
    contracts::{
        pool_manager::{
            pool_state::{pool_manager_pool_last_fee_growth_global, pool_manager_pool_state_slot},
            pool_tick_state::pool_manager_pool_tick_fee_growth_outside
        },
        utils::encode_position_key
    }
};

// position state
pub const POOL_MANAGER_POSITION_STATE_OFFSET_SLOT: u8 = 6;
pub const POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE0_LAST_X128_SLOT_OFFSET: u8 = 1;
pub const POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE1_LAST_X128_SLOT_OFFSET: u8 = 2;

pub fn pool_manager_position_state_slot(pool_id: U256, position_id: U256) -> B256 {
    let pools_slot = U256::from_be_slice(pool_manager_pool_state_slot(pool_id).as_slice())
        + U256::from(POOL_MANAGER_POSITION_STATE_OFFSET_SLOT);
    keccak256((position_id, pools_slot).abi_encode())
}

pub async fn pool_manager_position_fee_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    current_tick: I24,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<(U256, U256)> {
    let (
        (fee_growth_global0_x128, fee_growth_global1_x128),
        (tick_lower_fee_growth_outside0_x128, tick_lower_fee_growth_outside1_x128),
        (tick_upper_fee_growth_outside0_x128, tick_upper_fee_growth_outside1_x128)
    ) = tokio::try_join!(
        pool_manager_pool_last_fee_growth_global(
            slot_fetcher,
            pool_manager_address,
            block_number,
            pool_id,
        ),
        pool_manager_pool_tick_fee_growth_outside(
            slot_fetcher,
            pool_manager_address,
            block_number,
            pool_id,
            tick_lower
        ),
        pool_manager_pool_tick_fee_growth_outside(
            slot_fetcher,
            pool_manager_address,
            block_number,
            pool_id,
            tick_upper
        )
    )?;

    let (fee_growth_inside0_x128, fee_growth_inside1_x128) = if current_tick < tick_lower {
        (
            tick_lower_fee_growth_outside0_x128 - tick_upper_fee_growth_outside0_x128,
            tick_lower_fee_growth_outside1_x128 - tick_upper_fee_growth_outside1_x128
        )
    } else if current_tick >= tick_upper {
        (
            tick_upper_fee_growth_outside0_x128 - tick_lower_fee_growth_outside0_x128,
            tick_upper_fee_growth_outside1_x128 - tick_lower_fee_growth_outside1_x128
        )
    } else {
        (
            fee_growth_global0_x128
                - tick_lower_fee_growth_outside0_x128
                - tick_upper_fee_growth_outside0_x128,
            fee_growth_global1_x128
                - tick_lower_fee_growth_outside1_x128
                - tick_upper_fee_growth_outside1_x128
        )
    };

    Ok((fee_growth_inside0_x128, fee_growth_inside1_x128))
}

pub async fn pool_manager_position_state_last_fee_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<(U256, U256)> {
    let position_key = U256::from_be_slice(
        encode_position_key(position_token_id, tick_lower, tick_upper).as_slice()
    );
    let position_state_slot = pool_manager_position_state_slot(pool_id.into(), position_key);
    let position_state_slot_base = U256::from_be_slice(position_state_slot.as_slice());

    let fee_growth_inside0_last_x128_slot = position_state_slot_base
        + U256::from(POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE0_LAST_X128_SLOT_OFFSET);
    let fee_growth_inside1_last_x128_slot = position_state_slot_base
        + U256::from(POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE1_LAST_X128_SLOT_OFFSET);

    let (fee_growth_inside0_last_x128, fee_growth_inside1_last_x128) = tokio::try_join!(
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_inside0_last_x128_slot.into(),
            block_number
        ),
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_inside1_last_x128_slot.into(),
            block_number
        )
    )?;

    Ok((fee_growth_inside0_last_x128, fee_growth_inside1_last_x128))
}

pub async fn pool_manager_position_state_liquidity<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<u128> {
    let position_key = U256::from_be_slice(
        encode_position_key(position_token_id, tick_lower, tick_upper).as_slice()
    );
    let position_state_slot = pool_manager_position_state_slot(pool_id.into(), position_key);

    let liquidity = slot_fetcher
        .storage_at(pool_manager_address, position_state_slot, block_number)
        .await?;

    Ok(liquidity.to())
}

#[cfg(test)]
mod tests {

    use angstrom_types::{self, primitive::POOL_MANAGER_ADDRESS};

    use super::*;
    use crate::test_utils::valid_test_params::init_valid_position_params_with_provider;

    #[tokio::test]
    async fn test_pool_manager_position_fee_growth_inside() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let results = pool_manager_position_fee_growth_inside(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into(),
            pos_info.current_pool_tick,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        println!("{results:?}");

        // assert_eq!(results, pos_info.position_liquidity);
    }

    #[tokio::test]
    async fn test_pool_manager_position_state_last_fee_growth_inside() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let results = pool_manager_position_state_last_fee_growth_inside(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into(),
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        println!("{results:?}");

        // assert_eq!(results, pos_info.position_liquidity);
    }

    #[tokio::test]
    async fn test_pool_manager_position_state_liquidity() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = pool_manager_position_state_liquidity(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into(),
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        assert_eq!(results, pos_info.position_liquidity);
    }
}
