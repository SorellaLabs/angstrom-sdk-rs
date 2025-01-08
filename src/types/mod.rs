mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("8fc277D218fA5D4dD5AA789bA4F9EA9beFB583Af");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("f7C50262949Bf79d1802b8bEe5ea00617ac2EA80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("42804cd3ad68cCeaE66C080c15829654C7eA5e2b");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("cCdA376d6d39CB0B2c9ba328b6CC77d586e7B438");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
