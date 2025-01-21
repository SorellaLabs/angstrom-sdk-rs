mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const POSITION_FETCHER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("59167FBa92904D512a3b3A9005eE916AF6acA687");

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("3c7bF57FD2CEA97eC78e58803662978575Cf79ca");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("efa489c72885095170b02CA2d826c22FECB51a90");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("06fE3C4D1515C4Ff139C9E544035F847EC2b5e23");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("C310E4d9dBF28E5e184B802E978Ab5a8E1CfCb56");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
