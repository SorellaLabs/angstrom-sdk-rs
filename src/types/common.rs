use alloy_primitives::{
    Address, U256,
    aliases::{I24, U24},
};
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey,
    contract_payloads::angstrom::AngPoolConfigEntry, primitive::PoolId,
};

use serde::{Deserialize, Serialize};

use super::ANGSTROM_ADDRESS;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenPairInfo {
    pub token0: Address,
    pub token1: Address,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TokenInfoWithMeta {
    pub address: Address,
    pub symbol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetadata {
    pub pool_id: PoolId,
    pub token0: Address,
    pub token1: Address,
    pub fee: u32,
    pub tick_spacing: u16,
    pub storage_idx: u64,
}

impl PoolMetadata {
    pub fn new(token0: Address, token1: Address, config_store: AngPoolConfigEntry) -> Self {
        let pool_key = PoolKey {
            currency0: token0,
            currency1: token1,
            fee: U24::from(config_store.fee_in_e6),
            tickSpacing: I24::unchecked_from(config_store.tick_spacing),
            hooks: ANGSTROM_ADDRESS,
        };

        Self {
            token0,
            token1,
            pool_id: pool_key.into(),
            fee: config_store.fee_in_e6,
            tick_spacing: config_store.tick_spacing,
            storage_idx: config_store.store_index as u64,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TokensOrPoolId {
    Tokens(Address, Address),
    PoolId(PoolId),
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
    if token0 < token1 { (token0, token1) } else { (token1, token0) }
}

#[derive(Debug, Clone)]
pub struct UserLiquidityPosition {
    pub pool_id: PoolId,
    pub token_id: U256,
    pub tick_lower: i32,
    pub tick_upper: i32,
    pub pool_key: PoolKey,
}

impl UserLiquidityPosition {
    pub fn new(
        pool_key: PoolKey,
        position: angstrom_types::contract_bindings::position_fetcher::PositionFetcher::Position,
    ) -> Self {
        let pool_id = pool_key.clone().into();
        Self {
            pool_id,
            token_id: position.tokenId,
            tick_lower: position.tickLower.as_i32(),
            tick_upper: position.tickUpper.as_i32(),
            pool_key,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinanceTokenPrice {
    pub address: Address,
    pub price: Option<f64>,
    pub error_msg: Option<String>,
}
