use alloy_primitives::{
    aliases::{I24, U24},
    Address, U256
};
use alloy_rpc_types::TransactionRequest;
use angstrom_sdk_macros::NeonObject;
use angstrom_types::{
    contract_bindings::{angstrom::Angstrom::PoolKey, pool_gate::PoolGate},
    contract_payloads::angstrom::AngPoolConfigEntry,
    primitive::PoolId
};
#[cfg(feature = "neon")]
use neon::object::Object;
use serde::{Deserialize, Serialize};

use super::ANGSTROM_ADDRESS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub struct TokenPairInfo {
    pub token0:    Address,
    pub token1:    Address,
    pub is_active: bool
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetadata {
    // pub pool_ticker: String,
    pub pool_id:      PoolId,
    pub token0:       Address,
    pub token1:       Address,
    pub fee:          u32,
    pub tick_spacing: u16,
    pub storage_idx:  u64
}

impl PoolMetadata {
    pub fn new(token0: Address, token1: Address, config_store: AngPoolConfigEntry) -> Self {
        let pool_key = PoolKey {
            currency0:   token0,
            currency1:   token1,
            fee:         U24::from(config_store.fee_in_e6),
            tickSpacing: I24::unchecked_from(config_store.tick_spacing),
            hooks:       ANGSTROM_ADDRESS
        };

        Self {
            token0,
            token1,
            pool_id: pool_key.into(),
            fee: config_store.fee_in_e6,
            tick_spacing: config_store.tick_spacing,
            storage_idx: config_store.store_index as u64
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRequestWithLiquidityMeta {
    pub tx_request: TransactionRequest,
    pub token0:     Address,
    pub token1:     Address,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub liquidity:  U256,
    pub is_add:     bool
}

impl TransactionRequestWithLiquidityMeta {
    pub fn new_add_liqudity(
        tx_request: TransactionRequest,
        call: PoolGate::addLiquidityCall
    ) -> Self {
        Self {
            tx_request,
            token0: call.asset0,
            token1: call.asset1,
            tick_lower: call.tickLower.try_into().unwrap(),
            tick_upper: call.tickUpper.try_into().unwrap(),
            liquidity: call.liquidity,
            is_add: true
        }
    }

    pub fn new_remove_liqudity(
        tx_request: TransactionRequest,
        call: PoolGate::removeLiquidityCall
    ) -> Self {
        Self {
            tx_request,
            token0: call.asset0,
            token1: call.asset1,
            tick_lower: call.tickLower.try_into().unwrap(),
            tick_upper: call.tickUpper.try_into().unwrap(),
            liquidity: call.liquidity,
            is_add: false
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TokensOrPoolId {
    Tokens(Address, Address),
    PoolId(PoolId)
}

impl From<PoolId> for TokensOrPoolId {
    fn from(value: PoolId) -> Self {
        TokensOrPoolId::PoolId(value)
    }
}

impl From<(Address, Address)> for TokensOrPoolId {
    fn from(value: (Address, Address)) -> Self {
        let (t0, t1) = if value.0 > value.1 { (value.1, value.0) } else { (value.0, value.1) };

        TokensOrPoolId::Tokens(t0, t1)
    }
}

pub(crate) fn sort_tokens(token0: Address, token1: Address) -> (Address, Address) {
    if token0 < token1 {
        (token0, token1)
    } else {
        (token1, token0)
    }
}
