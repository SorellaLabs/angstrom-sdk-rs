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
-> (AlloyProviderWrapper<base_common_network::Base>, ValidPositionTestParameters) {
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

    // Owner at `block_for_liquidity_add`; the position was later transferred to
    // the multisig.
    let owner = address!("0xbb660CaA10c6b28BDf44E10351C7FDE561D87d6b");
    let pool_id = b256!("0x922154690ae4d86388bd85aa62e654ee7eb8c70d736025525b8dfcfd4d82eaa1");
    let hook_address = address!("0x02C17501E53fBB7EB0E243c74AfC6a9e01C265CF");
    let tick_lower = I24::unchecked_from(-200210);
    let tick_upper = I24::unchecked_from(-195100);
    let position_token_id = U256::from(3086401_u64);

    let position_manager_pool_map_key = [
        146, 33, 84, 105, 10, 228, 216, 99, 136, 189, 133, 170, 98, 230, 84, 238, 126, 184, 199,
        13, 115, 96, 37, 82, 91
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
        position_liquidity: 727070318961784,
        block_number: 51784313,
        current_pool_tick: I24::unchecked_from(-197374),
        tick_upper,
        position_manager_pool_map_key,
        owner,
        pool_key,
        block_for_liquidity_add: 51749508,
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
