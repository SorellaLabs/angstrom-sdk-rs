use alloy_primitives::{
    aliases::{I24, U24},
    Address,
};
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey,
    contract_payloads::angstrom::AngPoolConfigEntry, primitive::PoolId,
};
use serde::{Deserialize, Serialize};

use super::ANGSTROM_ADDRESS;

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct MarketContext {
//     pub tokens: Vec<TokenContext>,
//     pub universe: Vec<PoolContext>,
// }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPairInfo {
    pub token0: Address,
    pub token1: Address,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolMetadata {
    // pub pool_ticker: String,
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

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct TickerContext {
//     pub pool: PoolContext,
//     pub stats: PoolStats,
// }

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct PoolStats {}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct PoolCandle {}

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub enum CandleTimeframe {
//     OneMinute,
//     FifteenMinutes,
//     OneHour,
//     OneDay,
// }
