use alloy_primitives::{U256, aliases::I24};
use angstrom_types_primitives::contract_bindings::pool_manager::PoolManager::PoolKey;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserLiquidityPosition {
    pub token_id:   U256,
    pub tick_lower: I24,
    pub tick_upper: I24,
    pub liquidity:  u128,
    pub pool_key:   PoolKey
}
