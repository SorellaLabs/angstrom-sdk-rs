mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

mod config;
pub use config::*;

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("8c7AAa1d34ea02C2CaDe2B5d073F08AC9b099635");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("7be4551c5C8Fc657fC2bD0E26Bd040A8CD4EaA80");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("8eb07C10398eB1918FEF1b781A1a618fdbaF89cE");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("dC4AD216394e7C80BB2282E1fd12a0dFF3EA94Bb");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 2;
