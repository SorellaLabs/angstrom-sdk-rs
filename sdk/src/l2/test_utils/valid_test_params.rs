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

use crate::l2::{ANGSTROM_L2_CONSTANTS_BASE_MAINNET, AngstromL2Chain, test_utils::BASE_USDC};
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
    let pool_id = b256!("0x71deb282904d0f76bc8c7867f4618ff91dcb43cf4574bc64700ffc48791d369c");
    let hook_address = address!("0x631352Aaa9d6554848aF674106bCD8Bb9E59a5CF");
    let tick_lower = I24::unchecked_from(-203530);
    let tick_upper = I24::unchecked_from(-197310);
    let position_token_id = U256::from(1970005u64);

    let position_manager_pool_map_key = [
        113, 222, 178, 130, 144, 77, 15, 118, 188, 140, 120, 103, 244, 97, 143, 249, 29, 203, 67,
        207, 69, 116, 188, 100, 112
    ];

    let pool_key = PoolManager::PoolKey {
        currency0:   Address::ZERO,
        currency1:   BASE_USDC,
        fee:         U24::from(160),
        tickSpacing: I24::unchecked_from(10),
        hooks:       hook_address
    };

    ValidPositionTestParameters {
        pool_id,
        position_token_id,
        tick_lower,
        position_liquidity: 590304962892303,
        block_number: 43100000,
        current_pool_tick: I24::unchecked_from(-200640),
        tick_upper,
        position_manager_pool_map_key,
        owner,
        pool_key,
        block_for_liquidity_add: 42976298,
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
