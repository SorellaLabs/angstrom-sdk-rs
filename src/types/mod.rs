mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("c4A77030f43286026450E70d5f69e1f2eeD0Ad05");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("9388E7b5d27f69D8563d9bb09Da6982Bf0632a80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("16Bf33E75312c630d37b0537807E4AC3b47d52eA");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("5F488dE79C4bf3bB87BBeACB715b1682A1e0022B");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
