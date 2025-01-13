mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("801471E6752722AF45e41FcDB42ABD2d72a2a17E");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("B869e524a7b9E779Ba8524aD689cA23Cc555ea80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("B0e79f76E61B57fc6970576B5B8109d91bA5b41d");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("ED0C8E510859F2f534201403C884e65CACF2EFcF");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
