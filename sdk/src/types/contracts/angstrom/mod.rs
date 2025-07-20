use alloy_primitives::{Address, B256, U256, aliases::I24, keccak256};
use alloy_sol_types::SolValue;
use angstrom_types::primitive::PoolId;

use crate::types::{StorageSlotFetcher, contracts::utils::encode_position_key};

pub const ANGSTROM_POOL_REWARDS_GROWTH_ARRAY_SIZE: u64 = 16777216;
pub const BLOCKS_24HR: u64 = 7200;

pub const ANGSTROM_POSITIONS_SLOT: u8 = 6;
pub const ANGSTROM_POOL_REWARDS_SLOT: u8 = 7;

pub fn angstrom_position_slot(pool_id: PoolId, position_key: B256) -> B256 {
    let inner = keccak256((pool_id, U256::from(ANGSTROM_POSITIONS_SLOT)).abi_encode());
    keccak256((position_key, inner).abi_encode())
}

pub fn angstrom_pool_rewards_slot(pool_id: PoolId) -> B256 {
    keccak256((pool_id, U256::from(ANGSTROM_POOL_REWARDS_SLOT)).abi_encode())
}

pub async fn angstrom_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    current_pool_tick: I24,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<U256> {
    let (lower_growth, upper_growth, global_growth) = tokio::try_join!(
        angstrom_tick_growth_outside(
            slot_fetcher,
            angstrom_address,
            block_number,
            pool_id,
            tick_lower
        ),
        angstrom_tick_growth_outside(
            slot_fetcher,
            angstrom_address,
            block_number,
            pool_id,
            tick_upper
        ),
        angstrom_global_growth(slot_fetcher, angstrom_address, block_number, pool_id),
    )?;

    let rewards = if current_pool_tick < tick_lower {
        lower_growth - upper_growth
    } else if current_pool_tick >= tick_upper {
        upper_growth - lower_growth
    } else {
        global_growth - lower_growth - upper_growth
    };

    Ok(rewards)
}

pub async fn angstrom_global_growth<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId
) -> eyre::Result<U256> {
    let pool_rewards_slot_base = U256::from_be_bytes(angstrom_pool_rewards_slot(pool_id).0);
    let global_growth = slot_fetcher
        .storage_at(
            angstrom_address,
            (pool_rewards_slot_base + U256::from(ANGSTROM_POOL_REWARDS_GROWTH_ARRAY_SIZE)).into(),
            block_number
        )
        .await?;

    Ok(global_growth)
}

pub async fn angstrom_tick_growth_outside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    tick: I24
) -> eyre::Result<U256> {
    let pool_rewards_slot_base = U256::from_be_bytes(angstrom_pool_rewards_slot(pool_id).0);
    let global_growth = slot_fetcher
        .storage_at(
            angstrom_address,
            (pool_rewards_slot_base + U256::from_be_slice(&tick.to_be_bytes::<3>())).into(),
            block_number
        )
        .await?;

    Ok(global_growth)
}

pub async fn angstrom_last_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    position_token_id: U256,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<U256> {
    let position_key = encode_position_key(position_token_id, tick_lower, tick_upper);
    let position_slot_base = U256::from_be_bytes(angstrom_position_slot(pool_id, position_key).0);

    let growth = slot_fetcher
        .storage_at(angstrom_address, position_slot_base.into(), block_number)
        .await?;

    Ok(growth)
}

#[cfg(test)]
mod tests {

    use angstrom_types::primitive::ANGSTROM_ADDRESS;

    use super::*;
    use crate::test_utils::valid_test_params::init_valid_position_params_with_provider;

    #[tokio::test]
    async fn test_angstrom_growth_inside() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = angstrom_growth_inside(
            &provider,
            *ANGSTROM_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_id,
            pos_info.current_pool_tick,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        assert_eq!(results, U256::from(701740166379348581587029420336_u128))
    }

    #[tokio::test]
    async fn test_angstrom_last_growth_inside() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = angstrom_last_growth_inside(
            &provider,
            *ANGSTROM_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_id,
            pos_info.position_token_id,
            pos_info.tick_lower,
            pos_info.tick_upper
        )
        .await
        .unwrap();

        assert_eq!(results, U256::from(699096039990817998971892310067_u128))
    }

    #[tokio::test]
    async fn test_angstrom_global_growth() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = angstrom_global_growth(
            &provider,
            *ANGSTROM_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_id
        )
        .await
        .unwrap();

        assert_eq!(results, U256::from(701740166379348581587029420336_u128))
    }

    #[tokio::test]
    async fn test_angstrom_tick_growth_outside() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let results = angstrom_tick_growth_outside(
            &provider,
            *ANGSTROM_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_id,
            I24::unchecked_from(188250)
        )
        .await
        .unwrap();

        assert_eq!(results, U256::from(655382197592272071439615771424_u128))
    }
}
