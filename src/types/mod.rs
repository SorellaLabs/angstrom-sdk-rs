mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

mod config;
pub use config::*;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("3e23A27432444df19af7DF4751C5f2c079fD37FD");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("cd33550Fa13D627172c2E341d2EA9a9896fcEa80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("30EAD627Ff1b44B40a1Fcb54E27BEc2b399f5b3C");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("3ac4c9Ef19E9E9892daCe5B27a3ee4D9Be46D6aF");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
