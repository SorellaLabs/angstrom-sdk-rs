mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

mod config;
pub use config::*;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("962edDBDffDddaC3231092E2B5C2c52E28241e13");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("C9783b91002B176f6810F37Da17d0AD7b7E22A80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("e7B3dcF61A211fc1d4a3a098E4A35894A5E8CF0f");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("8818B69cBEC96b65782e2c3d853Fd188e4591Ef5");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
