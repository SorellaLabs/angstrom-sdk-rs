mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

mod config;
pub use config::*;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("58ba1dD9Fe664434D4E4092c6c2026984b896be3");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("207a00AD719F07B5Dc48129E05c0fe0fd4F2aA80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("f409Fad5AC3060521326F3b247BF54eDdEDd6EDa");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("9BcF03b86a1531EBe9D79dE22994aF3B60D53A07");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
