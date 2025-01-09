mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("9414F5757626C7ACa0AFe917F52E6c12527EC0ad");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("9a59a6d48aae9B192ac58871e112D9e441f86A80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("63757b2554C200A9b45892965D257D4a44427231");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("8C9dEB82c3a581d1e7c8b2c99bD04E22BDc37812");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
