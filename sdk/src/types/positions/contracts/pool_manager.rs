use alloy_primitives::{Address, B256, U256, aliases::I24, keccak256};
use alloy_sol_types::SolValue;
use angstrom_types::primitive::PoolId;

use crate::types::{
    StorageSlotFetcher,
    positions::utils::{UnpackSlot0, UnpackedSlot0, encode_position_key}
};

// pool state
pub const POOL_MANAGER_POOL_STATE_MAP_SLOT: u8 = 6;
pub const POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL0_X128_SLOT_OFFSET: u8 = 1;
pub const POOL_MANAGER_POOL_FEE_GROWTH_GLOBAL1_X128_SLOT_OFFSET: u8 = 2;

// tick state
pub const POOL_MANAGER_POOL_TICK_OFFSET_SLOT: u8 = 4;
pub const POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE0_X128_SLOT_OFFSET: u8 = 2;
pub const POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE1_X128_SLOT_OFFSET: u8 = 3;

// position state
pub const POOL_MANAGER_POSITION_STATE_OFFSET_SLOT: u8 = 6;
pub const POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE0_LAST_X128_SLOT_OFFSET: u8 = 1;
pub const POOL_MANAGER_POSITION_STATE_FEE_GROWTH_INSIDE1_LAST_X128_SLOT_OFFSET: u8 = 2;

pub fn pool_manager_pool_state_slot(pool_id: U256) -> B256 {
    keccak256((pool_id, U256::from(POOL_MANAGER_POOL_STATE_MAP_SLOT)).abi_encode())
}

pub fn pool_manager_position_state_slot(pool_id: U256, position_id: U256) -> B256 {
    let pools_slot = U256::from_be_slice(pool_manager_pool_state_slot(pool_id).as_slice())
        + U256::from(POOL_MANAGER_POSITION_STATE_OFFSET_SLOT);
    keccak256((position_id, pools_slot).abi_encode())
}

pub fn pool_manager_pool_tick_slot(pool_id: U256, tick: I24) -> B256 {
    let inner = U256::from_be_bytes(pool_manager_pool_state_slot(pool_id).0)
        + U256::from(POOL_MANAGER_POOL_TICK_OFFSET_SLOT);
    keccak256((tick, inner).abi_encode())
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
        .storage_at(pool_manager_address, pool_state_slot.into(), block_number)
        .await?;

    Ok(packed_slot0.unpack_slot0())
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

pub async fn pool_manager_pool_tick_fee_growth_outside<F: StorageSlotFetcher>(
    slot_fetcher: &F,
    pool_manager_address: Address,
    block_number: Option<u64>,
    pool_id: PoolId,
    tick: I24
) -> eyre::Result<(U256, U256)> {
    let pool_tick_slot = pool_manager_pool_tick_slot(pool_id.into(), tick);
    let pool_tick_slot_base = U256::from_be_slice(pool_tick_slot.as_slice());

    let fee_growth_outside0_x128_slot = pool_tick_slot_base
        + U256::from(POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE0_X128_SLOT_OFFSET);
    let fee_growth_outside1_x128_slot = pool_tick_slot_base
        + U256::from(POOL_MANAGER_POOL_TICK_FEE_GROWTH_OUTSIDE1_X128_SLOT_OFFSET);

    let (fee_growth_outside0_x128, fee_growth_outside1_x128) = tokio::try_join!(
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_outside0_x128_slot.into(),
            block_number
        ),
        slot_fetcher.storage_at(
            pool_manager_address,
            fee_growth_outside1_x128_slot.into(),
            block_number
        )
    )?;

    Ok((fee_growth_outside0_x128, fee_growth_outside1_x128))
}

#[cfg(test)]
mod tests {

    use alloy_primitives::{I256, U160, address, aliases::U24, b256};
    use angstrom_types::{
        contract_bindings::{
            angstrom::IPoolManager::ModifyLiquidityParams, pool_manager::PoolManager
        },
        primitive::{ANGSTROM_ADDRESS, POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS}
    };

    use super::*;
    use crate::{
        test_utils::valid_test_params::{
            init_valid_position_params_with_provider, mainnet_provider
        },
        types::positions::{
            angstrom_growth_inside, angstrom_last_growth_inside, position_manager_position_info
        }
    };

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

    #[tokio::test]
    async fn test_pool_manager_pool_state_last_fee_growth_global() {
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

        println!("{:?}", results);

        // assert_eq!(results, pos_info.position_liquidity);
    }

    #[tokio::test]
    async fn test_pool_manager_pool_tick_fee_growth_outside() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.valid_block_after_swaps;

        let results = pool_manager_pool_tick_fee_growth_outside(
            &provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            Some(block_number),
            pos_info.pool_key.into(),
            pos_info.current_pool_tick
        )
        .await
        .unwrap();

        println!("{:?}", results);

        // assert_eq!(results, pos_info.position_liquidity);
    }

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

        println!("{:?}", results);

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

    // #[tokio::test]
    // async fn test_uniswap_last_growth_inside_vuln() {
    //     let provider = mainnet_provider("ws://100.26.105.109:8546").await;
    //     let block_number = 22864946;

    //     let pool_key = PoolManager::PoolKey {
    //         currency0:
    // address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48"),
    //         currency1:
    // address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2"),         fee:
    // U24::from(8388608),         tickSpacing: I24::unchecked_from(10),
    //         hooks:
    // address!("0x0000000aa8c2fb9b232f78d2b286dc2ae53bfad4")     };

    //     let salt =
    // b256!("0x000000000000000000000000000000000000000000000000000000000000837a"
    // );     let params = ModifyLiquidityParams {
    //         tickLower: I24::unchecked_from(197910),
    //         tickUpper: I24::unchecked_from(198100),
    //         liquidityDelta: I256::unchecked_from(-15437641962586_i128),
    //         salt
    //     };

    //     let slot0 = pool_manager_pool_slot0(
    //         &provider,
    //         *POOL_MANAGER_ADDRESS.get().unwrap(),
    //         Some(block_number),
    //         pool_key.clone().into()
    //     )
    //     .await
    //     .unwrap();

    //     println!("{slot0:?}");

    //     let growth_inside = angstrom_growth_inside(
    //         &provider,
    //         *ANGSTROM_ADDRESS.get().unwrap(),
    //         Some(block_number),
    //         pool_key.clone().into(),
    //         slot0.tick,
    //         params.tickLower,
    //         params.tickUpper
    //     )
    //     .await
    //     .unwrap();

    //     println!("{growth_inside:?}");

    //     let last_growth_inside = angstrom_last_growth_inside(
    //         &provider,
    //         *ANGSTROM_ADDRESS.get().unwrap(),
    //         Some(block_number),
    //         pool_key.clone().into(),
    //         U256::from_be_slice(salt.as_slice()),
    //         params.tickLower,
    //         params.tickUpper
    //     )
    //     .await
    //     .unwrap();

    //     println!("{last_growth_inside:?}");

    //     let liquidity = U256::from(
    //         pool_manager_position_state_liquidity(
    //             &provider,
    //             *POOL_MANAGER_ADDRESS.get().unwrap(),
    //             Some(block_number),
    //             pool_key.clone().into(),
    //             U256::from_be_slice(salt.as_slice()),
    //             params.tickLower,
    //             params.tickUpper
    //         )
    //         .await
    //         .unwrap()
    //     );
    // }
}
