mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("D1ED1800c848EC8bdc18E1e93EEC1E0128b6c821");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("591BA8DB07765de124ea9fa60109C2Dd6B49EA80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("722214aa466db5f2CEAa2AE7f6Ad7b7a326E97EB");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("3E81e60E632aa36782c22Dc231cA4Cf84eE70ae1");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
