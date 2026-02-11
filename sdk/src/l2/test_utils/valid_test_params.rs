use alloy_primitives::{
    Address, U256, address,
    aliases::{I24, U24},
    b256
};
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager,
    primitive::{PoolId, try_init_with_chain_id}
};
use uniswap_storage::v4::UnpackedPositionInfo;

use crate::l2::{
    ANGSTROM_L2_CONSTANTS_BASE_MAINNET, AngstromL2Chain,
    test_utils::{BASE_USDC, BASE_WETH}
};
#[cfg(feature = "local-reth")]
use crate::types::BaseMainnetExt;
#[cfg(not(feature = "local-reth"))]
use crate::types::providers::AlloyProviderWrapper;

pub struct ValidPositionTestParameters {
    pub owner: Address,
    pub pool_id: PoolId,
    pub pool_key: PoolManager::PoolKey,
    pub current_pool_tick: I24,
    pub position_manager_pool_map_key: [u8; 25],
    pub position_token_id: U256,
    pub tick_lower: I24,
    pub tick_upper: I24,
    pub position_liquidity: u128,
    pub block_number: u64,
    pub block_for_liquidity_add: u64,
    pub chain: AngstromL2Chain
}

#[cfg(not(feature = "local-reth"))]
pub async fn init_valid_position_params_with_provider()
-> (AlloyProviderWrapper<op_alloy_network::Optimism>, ValidPositionTestParameters) {
    let params = init_valid_position_params();
    let provider = crate::l2::test_utils::eth_provider().await.unwrap();

    (AlloyProviderWrapper::new(provider), params)
}

#[cfg(feature = "local-reth")]
pub async fn init_valid_position_params_with_provider() -> (
    std::sync::Arc<crate::types::providers::RethDbProviderWrapper<BaseMainnetExt>>,
    ValidPositionTestParameters
) {
    use std::sync::Arc;

    use lib_reth::{op_reth::BASE_MAINNET, reth_libmdbx::RethNodeClientBuilder};

    use crate::{l2::test_utils::base_eth_ws_url, types::providers::RethDbProviderWrapper};

    let params = init_valid_position_params();
    let provider = Arc::new(RethDbProviderWrapper::new(Arc::new(
        RethNodeClientBuilder::new(
            "/var/lib/eth/base-mainnet/reth/",
            1000,
            BASE_MAINNET.clone(),
            Some(&base_eth_ws_url())
        )
        .build()
        .unwrap()
    )));

    (provider, params)
}

pub fn init_valid_position_params() -> ValidPositionTestParameters {
    let chain_consts = ANGSTROM_L2_CONSTANTS_BASE_MAINNET;
    let _ = try_init_with_chain_id(chain_consts.chain_id());

    let owner = address!("0xe344c3d419B7788006ab5aF4355E03b04CE75579");
    let pool_id = b256!("0xe500210c7ea6bfd9f69dce044b09ef384ec2b34832f132baec3b418208e3a657");
    let tick_lower = I24::unchecked_from(194970);
    let tick_upper = I24::unchecked_from(198000);
    let position_token_id = U256::from(102303_u128);

    let position_manager_pool_map_key = [
        229, 0, 33, 12, 126, 166, 191, 217, 246, 157, 206, 4, 75, 9, 239, 56, 78, 194, 179, 72, 50,
        241, 50, 186, 236
    ];

    let angstrom_address = Address::random();
    let pool_key = PoolManager::PoolKey {
        currency0:   BASE_USDC,
        currency1:   BASE_WETH,
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
        block_for_liquidity_add: 23871281,
        chain: AngstromL2Chain::Base
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
