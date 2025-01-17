mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0xd99a23ECF12FD411660Be733caAe736777206011");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0x611362c23b013D1ad4Be4C018D8Ad2842d876a80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0x06fE3C4D1515C4Ff139C9E544035F847EC2b5e23");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0xC310E4d9dBF28E5e184B802E978Ab5a8E1CfCb56");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
