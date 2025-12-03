use alloy_primitives::{
    Address, TxHash, U256, address,
    aliases::{I24, U24},
    b256
};
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager,
    primitive::{
        ANGSTROM_ADDRESS, CONTROLLER_V1_ADDRESS, POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS,
        PoolId, try_init_with_chain_id
    }
};
use uniswap_storage::v4::UnpackedPositionInfo;

use crate::test_utils::{USDC, WETH};

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

#[cfg(not(feature = "local-reth"))]
pub async fn init_valid_position_params_with_provider()
-> (alloy_provider::RootProvider, ValidPositionTestParameters) {
    use alloy_provider::Provider;

    let params = init_valid_position_params();
    let provider = crate::test_utils::spawn_angstrom_api()
        .await
        .unwrap()
        .eth_provider()
        .clone()
        .root()
        .clone();

    (provider, params)
}

#[cfg(feature = "local-reth")]
pub async fn init_valid_position_params_with_provider() -> (
    std::sync::Arc<crate::providers::local_reth::RethDbProviderWrapper>,
    ValidPositionTestParameters
) {
    use std::sync::Arc;

    use lib_reth::{MAINNET, reth_libmdbx::RethNodeClientBuilder};

    use crate::{providers::local_reth::RethDbProviderWrapper, test_utils::eth_ws_url};

    let params = init_valid_position_params();
    let provider = Arc::new(RethDbProviderWrapper::new(Arc::new(
        RethNodeClientBuilder::new(
            "/var/lib/eth/mainnet/reth/",
            1000,
            MAINNET.clone(),
            Some(eth_ws_url())
        )
        .build()
        .unwrap()
    )));

    (provider, params)
}

pub fn init_valid_position_params() -> ValidPositionTestParameters {
    let _ = try_init_with_chain_id(1);

    let owner = address!("0xe344c3d419B7788006ab5aF4355E03b04CE75579");
    let pool_id = b256!("0xe500210c7ea6bfd9f69dce044b09ef384ec2b34832f132baec3b418208e3a657");
    let tick_lower = I24::unchecked_from(194970);
    let tick_upper = I24::unchecked_from(198000);
    let position_token_id = U256::from(102303_u128);

    let position_manager_pool_map_key = [
        229, 0, 33, 12, 126, 166, 191, 217, 246, 157, 206, 4, 75, 9, 239, 56, 78, 194, 179, 72, 50,
        241, 50, 186, 236
    ];

    let angstrom_address = *ANGSTROM_ADDRESS.get().unwrap();
    let pool_key = PoolManager::PoolKey {
        currency0:   USDC,
        currency1:   WETH,
        fee:         U24::from(0x800000),
        tickSpacing: I24::unchecked_from(10),
        hooks:       angstrom_address
    };

    ValidPositionTestParameters {
        pool_id,
        position_token_id,
        tick_lower,
        position_liquidity: 807449445327074,
        block_number: 23870000,
        current_pool_tick: I24::unchecked_from(196699),
        tick_upper,
        position_manager_pool_map_key,
        owner,
        pool_key,
        angstrom_address,
        block_for_liquidity_add: 23871281,
        valid_block_after_swaps: 23870004,
        pool_manager_address: *POOL_MANAGER_ADDRESS.get().unwrap(),
        position_manager_address: *POSITION_MANAGER_ADDRESS.get().unwrap(),
        controller_v1_address: *CONTROLLER_V1_ADDRESS.get().unwrap(),
        bundle_tx_hash: b256!("0x0e154cbadc0af7195c7cd7fbb7110e68c79d1e453d2c6e315b3f6c4225f0dc79")
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
