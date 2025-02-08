mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const POSITION_FETCHER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("c9D3CC2DBf823D66400D952e2984e47C07E2DF68");

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("59167FBa92904D512a3b3A9005eE916AF6acA687");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("293954613283cC7B82BfE9676D3cc0fb0A58fAa0");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("F967Ede45ED04ec89EcA04a4c7175b6E0106e3A8");

pub const POSITION_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("EECc919927FeA22488D460d87De461809AaCbbA7");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("C310E4d9dBF28E5e184B802E978Ab5a8E1CfCb56");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 3;

pub const BINANCE_REST_API_BASE_URL: &str = "https://api.binance.us";
