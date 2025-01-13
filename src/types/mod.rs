mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("D316698A0231aAE7a55621238985cb2542637155");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("D007CB9Ab0cA5b0A2EBb3Dd4F8a9DAf0D5632a80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("5B9fE8c111a5F03fe0768c102e24fdc8fe3fe5C1");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("50c26EcfeeB6b38F82a2b32d4b517DB001989204");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
