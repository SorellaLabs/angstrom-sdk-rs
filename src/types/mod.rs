mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("8a1575eF61d7152d73Cf00f7Df34d592628c606A");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("26f209526dc896E3A69B761a26161791b7e3AA80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("3943468c83a2f8e825822Be392c27c14e8aF442F");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("404cd02a2F55581A04B54989AdE95beF30bcEe19");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
