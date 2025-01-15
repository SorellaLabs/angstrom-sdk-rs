mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("F25FdCb38AA295E68F30C42b33180c34b859dB49");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("1838fa3d964B0A751cA524C3eF8bec9d23932A80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("88569028ABFD769bBb2313cB69C1e3756AB7172E");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("Cfde10Ba0784D6dF84D252ED370Fa4faD6C271b3");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
