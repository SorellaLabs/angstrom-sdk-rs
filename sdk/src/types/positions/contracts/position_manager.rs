use alloy_primitives::{
    Address, B256, Bytes, U256,
    aliases::{I24, U24},
    keccak256
};
use alloy_sol_types::SolValue;
use angstrom_types::contract_bindings::pool_manager::PoolManager::PoolKey;
use itertools::concat;

use crate::types::{
    StorageSlotFetcher,
    positions::{UnpackPositionInfo, UnpackedPositionInfo}
};

pub const POSITION_MANAGER_OWNER_OF_SLOT: u8 = 2;
pub const POSITION_MANAGER_NEXT_TOKEN_ID_SLOT: u8 = 8;
pub const POSITION_MANAGER_POSITION_INFO_SLOT: u8 = 9;
pub const POSITION_MANAGER_POOL_KEYS_SLOT: u8 = 10;

pub fn position_manager_owner_of_slot(token_id: U256) -> B256 {
    keccak256((token_id, U256::from(POSITION_MANAGER_OWNER_OF_SLOT)).abi_encode())
}

pub fn position_manager_position_info_slot(token_id: U256) -> B256 {
    keccak256((token_id, U256::from(POSITION_MANAGER_POSITION_INFO_SLOT)).abi_encode())
}

pub fn position_manager_pool_key_and_info_slot(position_info: U256) -> B256 {
    let position_id = position_info.position_manager_pool_map_key();
    keccak256((position_id, U256::from(POSITION_MANAGER_POOL_KEYS_SLOT)).abi_encode())
}

pub async fn position_manager_position_info<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    position_manager_address: Address,
    block_number: Option<u64>,
    token_id: U256
) -> eyre::Result<U256> {
    let position_info_slot = position_manager_position_info_slot(token_id);

    let position_info = slot_fetcher
        .storage_at(position_manager_address, position_info_slot, block_number)
        .await?;

    Ok(position_info)
}

pub async fn position_manager_pool_key_and_info<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    position_manager_address: Address,
    block_number: Option<u64>,
    token_id: U256
) -> eyre::Result<(PoolKey, UnpackedPositionInfo)> {
    let position_info = position_manager_position_info(
        slot_fetcher,
        position_manager_address,
        block_number,
        token_id
    )
    .await?;
    let pool_key_slot_base =
        U256::from_be_slice(position_manager_pool_key_and_info_slot(position_info).as_slice());

    let (slot0, slot1, slot2) = tokio::try_join!(
        slot_fetcher.storage_at(position_manager_address, pool_key_slot_base.into(), block_number),
        slot_fetcher.storage_at(
            position_manager_address,
            (pool_key_slot_base + U256::from(1_u8)).into(),
            block_number
        ),
        slot_fetcher.storage_at(
            position_manager_address,
            (pool_key_slot_base + U256::from(2_u8)).into(),
            block_number
        )
    )?;

    let concatted_bytes = Bytes::from(concat([
        slot0.to_be_bytes_vec(),
        slot1.to_be_bytes_vec(),
        slot2.to_be_bytes_vec()
    ]));

    let currency0 = Address::from_slice(&concatted_bytes[12..32]);
    let currency1 = Address::from_slice(&concatted_bytes[44..64]);
    let fee = U24::from_be_slice(&concatted_bytes[41..44]);
    let tick_spacing = I24::try_from_be_slice(&concatted_bytes[38..41]).unwrap();
    let hooks = Address::from_slice(&concatted_bytes[76..96]);

    let pool_key = PoolKey { currency0, currency1, fee, tickSpacing: tick_spacing, hooks };

    Ok((pool_key, position_info.unpack_position_info()))
}

pub async fn position_manager_owner_of<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    position_manager_address: Address,
    block_number: Option<u64>,
    token_id: U256
) -> eyre::Result<Address> {
    let owner_of_slot = position_manager_owner_of_slot(token_id);

    let owner_of = slot_fetcher
        .storage_at(position_manager_address, owner_of_slot, block_number)
        .await?;

    Ok(Address::from_slice(&owner_of.to_be_bytes_vec()[12..32]))
}

pub async fn position_manager_next_token_id<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    position_manager_address: Address,
    block_number: Option<u64>
) -> eyre::Result<U256> {
    let next_token_id = slot_fetcher
        .storage_at(
            position_manager_address,
            U256::from(POSITION_MANAGER_NEXT_TOKEN_ID_SLOT).into(),
            block_number
        )
        .await?;

    Ok(next_token_id)
}

#[cfg(test)]
mod tests {

    use angstrom_types::primitive::{POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS};

    use super::*;
    use crate::test_utils::valid_test_params::init_valid_position_params_with_provider;

    #[tokio::test]
    async fn test_position_manager_position_info() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = position_manager_position_info(
            &provider,
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.position_token_id
        )
        .await
        .unwrap();

        let expected = U256::from_str_radix(
            "36752956352201235409813682138304141020772237719769761638105745524212318476800",
            10
        )
        .unwrap();
        assert_eq!(results, expected);
    }

    #[tokio::test]
    async fn test_position_manager_pool_key_and_info() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let (results, _) = position_manager_pool_key_and_info(
            &provider,
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.position_token_id
        )
        .await
        .unwrap();

        assert_eq!(results, pos_info.pool_key);
    }

    #[tokio::test]
    async fn test_position_manager_owner_of() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = position_manager_owner_of(
            &provider,
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.position_token_id
        )
        .await
        .unwrap();

        assert_eq!(results, pos_info.owner);
    }

    #[tokio::test]
    async fn test_position_manager_next_token_id() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_number;

        let results = position_manager_next_token_id(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number)
        )
        .await
        .unwrap();

        assert_eq!(results, U256::ZERO);
    }
}
