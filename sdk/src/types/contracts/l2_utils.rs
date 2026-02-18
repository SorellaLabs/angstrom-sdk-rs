use alloy_primitives::keccak256;
use alloy_sol_types::SolValue;
use angstrom_types_primitives::PoolId;

use crate::types::contracts::angstrom_l2::angstrom_l_2_factory::AngstromL2Factory;

impl Copy for AngstromL2Factory::PoolKey {}

impl From<AngstromL2Factory::PoolKey> for PoolId {
    fn from(value: AngstromL2Factory::PoolKey) -> Self {
        keccak256(value.abi_encode())
    }
}

impl From<&AngstromL2Factory::PoolKey> for PoolId {
    fn from(value: &AngstromL2Factory::PoolKey) -> Self {
        keccak256(value.abi_encode())
    }
}

#[cfg(feature = "l1")]
impl From<AngstromL2Factory::PoolKey>
    for angstrom_types_primitives::contract_bindings::pool_manager::PoolManager::PoolKey
{
    fn from(value: AngstromL2Factory::PoolKey) -> Self {
        Self {
            currency0:   value.currency0,
            currency1:   value.currency1,
            fee:         value.fee,
            tickSpacing: value.tickSpacing,
            hooks:       value.hooks
        }
    }
}
