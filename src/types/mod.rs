mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0B0131C3034AC999882685Bf62453BA52b829cE2");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("70Afa1658638f47D872E60bAC618e67084DAaa80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("25E4bbBC5aaa170532927E3AEBF1847Ae3290856");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("cD2AE6813D20E8f21cf8CdFc4b43868c9eFdC41A");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
