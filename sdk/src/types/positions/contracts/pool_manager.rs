use alloy_primitives::{Address, B256, U256, aliases::I24, keccak256};
use alloy_sol_types::SolValue;
use angstrom_types::{contract_bindings::pool_manager::PoolManager::PoolKey, primitive::PoolId};

use crate::types::{StorageSlotFetcher, positions::utils::encode_position_key};

pub const POOL_MANAGER_POOLS_SLOT: u8 = 6;
pub const POOL_MANAGER_POSITIONS_OFFSET_SLOT: u8 = 6;

pub fn pool_manager_pools_slot(pool_id: U256) -> B256 {
    keccak256((pool_id, U256::from(POOL_MANAGER_POOLS_SLOT)).abi_encode())
}

pub fn pool_manager_position_state_slot(pool_id: U256, position_id: U256) -> B256 {
    let pools_slot = U256::from_be_slice(pool_manager_pools_slot(pool_id).as_slice())
        + U256::from(POOL_MANAGER_POSITIONS_OFFSET_SLOT);
    keccak256((position_id, pools_slot).abi_encode())
}

pub async fn pool_manager_liquidity<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_key: PoolKey,
    token_id: U256,
    tick_lower: I24,
    tick_upper: I24
) -> eyre::Result<u128> {
    let position_key =
        U256::from_be_slice(encode_position_key(token_id, tick_lower, tick_upper).as_slice());
    let position_slot = pool_manager_position_state_slot(
        U256::from_be_slice(PoolId::from(pool_key).as_slice()),
        position_key
    );

    let liquidity = slot_fetcher
        .storage_at(pool_manager_address, position_slot, block_number)
        .await?;

    Ok(liquidity.to())
}

#[cfg(test)]
mod tests {
    use angstrom_types::primitive::POOL_MANAGER_ADDRESS;

    use super::*;
    use crate::test_utils::valid_test_params::init_valid_position_params_with_provider;

    #[tokio::test]
    async fn test_pool_manager_liquidity() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = pool_manager_liquidity(
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

        assert_eq!(results, pos_info.position_liquidity);
    }
}
