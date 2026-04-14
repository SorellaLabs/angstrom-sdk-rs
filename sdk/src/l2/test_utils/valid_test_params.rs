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

use crate::l2::{ANGSTROM_L2_CONSTANTS_BASE_MAINNET, AngstromL2Chain, test_utils::BASE_CB_BTC};
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
            Some(&base_eth_ws_url()),
            None
        )
        .build()
        .unwrap()
    )));

    (provider, params)
}

pub fn init_valid_position_params() -> ValidPositionTestParameters {
    let chain_consts = ANGSTROM_L2_CONSTANTS_BASE_MAINNET;
    let _ = try_init_with_chain_id(chain_consts.chain_id());

    let owner = address!("0x2A49fF6D0154506D0e1Eda03655F274126ceF7B6");
    let pool_id = b256!("0xd12d3ba76b3dccd9a551f5186771d9d4fed28a6612beb007f322a816f91a2e7a");
    let hook_address = address!("0x7Fa49D29481b6D168505Ccde26635e204c09e5CF");
    let tick_lower = I24::unchecked_from(-267180);
    let tick_upper = I24::unchecked_from(-263520);
    let position_token_id = U256::from(2092345_u64);

    let position_manager_pool_map_key = [
        209, 45, 59, 167, 107, 61, 204, 217, 165, 81, 245, 24, 103, 113, 217, 212, 254, 210, 138, 102, 18, 190, 176, 7, 243
    ];

    let pool_key = PoolManager::PoolKey {
        currency0:   Address::ZERO,
        currency1:   BASE_CB_BTC,
        fee:         U24::from(160),
        tickSpacing: I24::unchecked_from(60),
        hooks:       hook_address
    };

    ValidPositionTestParameters {
        pool_id,
        position_token_id,
        tick_lower,
        position_liquidity: 41433601053552,
        block_number: 43879800,
        current_pool_tick: I24::unchecked_from(-265348),
        tick_upper,
        position_manager_pool_map_key,
        owner,
        pool_key,
        block_for_liquidity_add: 43879728,
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
