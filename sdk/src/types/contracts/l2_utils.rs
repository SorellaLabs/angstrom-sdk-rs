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
