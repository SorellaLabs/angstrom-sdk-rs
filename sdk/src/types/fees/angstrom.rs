use alloy_primitives::{Address, B256, U256, aliases::I24, keccak256};
use alloy_sol_types::SolValue;
use angstrom_types::primitive::PoolId;

use crate::types::fees::StorageSlotFetcher;

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

pub async fn angstrom_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    tick_lower: I24,
    tick_upper: I24,
    current_tick: I24
) -> eyre::Result<U256> {
    let pool_rewards_slot_base = U256::from_be_bytes(angstrom_pool_rewards_slot(pool_id).0);

    let (lower_growth, upper_growth) = tokio::try_join!(
        slot_fetcher.storage_at(
            angstrom_address,
            (pool_rewards_slot_base + U256::from_be_slice(&tick_lower.to_be_bytes::<3>())).into(),
            block_number
        ),
        slot_fetcher.storage_at(
            angstrom_address,
            (pool_rewards_slot_base + U256::from_be_slice(&tick_upper.to_be_bytes::<3>())).into(),
            block_number
        ),
    )?;

    let global_growth = slot_fetcher
        .storage_at(
            angstrom_address,
            (pool_rewards_slot_base + U256::from(ANGSTROM_POOL_REWARDS_GROWTH_ARRAY_SIZE)).into(),
            block_number
        )
        .await?;

    let rewards = if current_tick < tick_lower {
        lower_growth - upper_growth
    } else if tick_upper <= current_tick {
        upper_growth - lower_growth
    } else {
        global_growth - lower_growth - upper_growth
    };

    Ok(rewards)
}

pub async fn angstrom_last_growth_inside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    angstrom_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    position_key: B256
) -> eyre::Result<U256> {
    let position_slot_base = U256::from_be_bytes(angstrom_position_slot(pool_id, position_key).0);

    let growth = slot_fetcher
        .storage_at(angstrom_address, position_slot_base.into(), block_number)
        .await?;

    Ok(growth)
}

#[cfg(test)]
mod tests {

    use alloy_primitives::{aliases::I24, bytes};
    use angstrom_types::{
        contract_bindings::position_manager::PositionManager,
        primitive::{ANGSTROM_ADDRESS, POSITION_MANAGER_ADDRESS}
    };

    use super::*;
    use crate::{
        apis::{AngstromDataApi, utils::view_call},
        test_utils::valid_test_params::init_valid_position_params_with_provider
    };

    #[tokio::test]
    async fn test_angstrom_position_rewards() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let position_liquidity = view_call(
            &provider,
            Some(block_number),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            PositionManager::getPositionLiquidityCall {
                tokenId: U256::from(pos_info.position_token_id)
            }
        )
        .await
        .unwrap()
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
            U256::from_be_slice(&bytes!("0x01612792b3556065f1b770b4720f02631af1").0)
        );
    }

    #[tokio::test]
    async fn test_angstrom_growth_inside() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let (_, pool) = provider
            .pool_data_by_pool_id(pos_info.pool_id, Some(block_number))
            .await
            .unwrap();

        let results = angstrom_growth_inside(
            &provider,
            *ANGSTROM_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_id,
            pos_info.tick_lower,
            pos_info.tick_upper,
            I24::unchecked_from(pool.tick)
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
            pos_info.angstrom_rewards_position_key
        )
        .await
        .unwrap();

        assert_eq!(results, U256::from(699096039990817998971892310067_u128))
    }
}
