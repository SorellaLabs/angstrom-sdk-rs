mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("20Cc09ac7E8d13Dd39177786f4f4e9a802fe69a3");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("9a59a6d48aae9B192ac58871e112D9e441f86A80");

pub const ANGSTROM_ADDRESS_SALT: u64 = 11412;

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("cD7AB7cFd92481C6AfF1E79F90A3Ac6056bd7A6e");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("691841C2B3b60c309ad7D97813bE591412b87167");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
