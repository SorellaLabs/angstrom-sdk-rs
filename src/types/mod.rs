mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("35eDCfa6E26648b71DB786448BA682683d428D43");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("3Ed25678AD59238476592C144457579e81F2EA80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("Abcf3464781384E9aD47BdcED34e5B91Ebf6E877");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("9b40ce40A76819B83822502DaF6Aefa63D6b8a31");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
