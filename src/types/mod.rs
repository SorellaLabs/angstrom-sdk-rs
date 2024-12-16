mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

mod config;
pub use config::*;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("e5aCa092d5e2e3De7Cc44fFEdb972ec9fC98B508");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("7417f1eC45ED81aA4217DE53324eC1Fe7eE22A80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("99e9E21C1909AB06d63372179B43e1c91645fb71");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("b6816Cda17a80aFB1D161Cfe5f577951b9DCFCFb");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
