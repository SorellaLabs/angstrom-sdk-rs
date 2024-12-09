mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

// mod sol;
// pub use sol::*;

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0000000000000000000000000000000000000000");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0000000000000000000000000000000000000000");

pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
