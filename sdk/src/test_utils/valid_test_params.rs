use alloy_primitives::{
    Address, B256, U256, address,
    aliases::{I24, U24},
    b256
};
use alloy_provider::{Provider, RootProvider};
use angstrom_types::{
    contract_bindings::position_manager::PositionManager,
    primitive::{
        ANGSTROM_ADDRESS, CONTROLLER_V1_ADDRESS, POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS,
        PoolId, try_init_with_chain_id
    }
};

use crate::{
    test_utils::spawn_angstrom_api,
    types::positions::{UnpackedPositionInfo, encode_angstrom_rewards_position_key}
};

pub struct ValidPositionTestParameters {
    pub angstrom_address: Address,
    pub pool_manager_address: Address,
    pub position_manager_address: Address,
    pub controller_v1_address: Address,
    pub owner: Address,
    pub pool_id: PoolId,
    pub pool_key: PositionManager::PoolKey,
    pub position_manager_pool_map_key: [u8; 25],
    pub position_token_id: U256,
    pub angstrom_rewards_position_key: B256,
    pub tick_lower: I24,
    pub tick_upper: I24,
    pub position_liquidity: u128,
    pub block_number: u64
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

    let owner = address!("0x429ba70129df741B2Ca2a85BC3A2a3328e5c09b4");
    let pool_id = b256!("0x51416fa593479e6932829c5baea2984cb14a28ce753789e361ef3799a8ee7e5c");
    let tick_lower = I24::unchecked_from(-887270);
    let tick_upper = I24::unchecked_from(887270);
    let position_token_id = U256::from(14328_u64);

    let angstrom_rewards_position_key =
        encode_angstrom_rewards_position_key(owner, position_token_id, tick_lower, tick_upper);

    let position_manager_pool_map_key = [
        81, 65, 111, 165, 147, 71, 158, 105, 50, 130, 156, 91, 174, 162, 152, 76, 177, 74, 40, 206,
        117, 55, 137, 227, 97
    ];

    let angstrom_address = *ANGSTROM_ADDRESS.get().unwrap();
    let pool_key = PositionManager::PoolKey {
        currency0:   address!("0x1c7d4b196cb0c7b01d743fbc6116a902379c7238"),
        currency1:   address!("0xfff9976782d46cc05630d1f6ebab18b2324d6b14"),
        fee:         U24::from(0x800000),
        tickSpacing: I24::unchecked_from(10),
        hooks:       angstrom_address
    };

    ValidPositionTestParameters {
        pool_id,
        angstrom_rewards_position_key,
        position_token_id,
        tick_lower,
        position_liquidity: 45448764343813,
        block_number: 8642854,
        tick_upper,
        position_manager_pool_map_key,
        owner,
        pool_key,
        angstrom_address,
        pool_manager_address: *POOL_MANAGER_ADDRESS.get().unwrap(),
        position_manager_address: *POSITION_MANAGER_ADDRESS.get().unwrap(),
        controller_v1_address: *CONTROLLER_V1_ADDRESS.get().unwrap()
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
