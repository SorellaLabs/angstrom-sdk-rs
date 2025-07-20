use alloy_primitives::{
    Address, TxHash, U256, address,
    aliases::{I24, U24},
    b256
};
use alloy_provider::{Provider, RootProvider};
use angstrom_types::{
    contract_bindings::pool_manager::PoolManager,
    primitive::{
        ANGSTROM_ADDRESS, CONTROLLER_V1_ADDRESS, POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS,
        PoolId, try_init_with_chain_id
    }
};

use crate::{test_utils::spawn_angstrom_api, types::contracts::UnpackedPositionInfo};

pub struct ValidPositionTestParameters {
    pub angstrom_address: Address,
    pub pool_manager_address: Address,
    pub position_manager_address: Address,
    pub controller_v1_address: Address,
    pub owner: Address,
    pub pool_id: PoolId,
    pub pool_key: PoolManager::PoolKey,
    pub current_pool_tick: I24,
    pub position_manager_pool_map_key: [u8; 25],
    pub position_token_id: U256,
    pub tick_lower: I24,
    pub tick_upper: I24,
    pub position_liquidity: u128,
    pub bundle_tx_hash: TxHash,
    pub block_number: u64,
    pub block_for_liquidity_add: u64,
    pub valid_block_after_swaps: u64
}

pub async fn init_valid_position_params_with_provider()
-> (RootProvider, ValidPositionTestParameters) {
    let params = init_valid_position_params();
    let provider = spawn_angstrom_api()
        .await
        .unwrap()
        .eth_provider()
        .clone()
        .root()
        .clone();

    (provider, params)
}

pub fn init_valid_position_params() -> ValidPositionTestParameters {
    let _ = try_init_with_chain_id(11155111);

    let owner = address!("0x247bcb856d028d66bd865480604f45797446d179");
    let pool_id = b256!("0x51416fa593479e6932829c5baea2984cb14a28ce753789e361ef3799a8ee7e5c");
    let tick_lower = I24::unchecked_from(-887270);
    let tick_upper = I24::unchecked_from(887270);
    let position_token_id = U256::from(14328_u64);

    let position_manager_pool_map_key = [
        81, 65, 111, 165, 147, 71, 158, 105, 50, 130, 156, 91, 174, 162, 152, 76, 177, 74, 40, 206,
        117, 55, 137, 227, 97
    ];

    let angstrom_address = *ANGSTROM_ADDRESS.get().unwrap();
    let pool_key = PoolManager::PoolKey {
        currency0:   address!("0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"),
        currency1:   address!("0xfff9976782d46cc05630d1f6ebab18b2324d6b14"),
        fee:         U24::from(0x800000),
        tickSpacing: I24::unchecked_from(10),
        hooks:       angstrom_address
    };

    ValidPositionTestParameters {
        pool_id,
        position_token_id,
        tick_lower,
        position_liquidity: 45448764343813,
        block_number: 8642854,
        current_pool_tick: I24::unchecked_from(190088),
        tick_upper,
        position_manager_pool_map_key,
        owner,
        pool_key,
        angstrom_address,
        block_for_liquidity_add: 8642751,
        valid_block_after_swaps: 8678399,
        pool_manager_address: *POOL_MANAGER_ADDRESS.get().unwrap(),
        position_manager_address: *POSITION_MANAGER_ADDRESS.get().unwrap(),
        controller_v1_address: *CONTROLLER_V1_ADDRESS.get().unwrap(),
        bundle_tx_hash: b256!("0x9ce27e5b59826ceab910d59a526ad9cdb23df71adff51df9bca82e24577c458a")
    }
}

impl ValidPositionTestParameters {
    pub fn as_unpacked_position_info(&self) -> UnpackedPositionInfo {
        UnpackedPositionInfo {
            position_manager_pool_map_key: self.position_manager_pool_map_key,
            tick_lower:                    self.tick_lower,
            tick_upper:                    self.tick_upper
        }
    }
}
