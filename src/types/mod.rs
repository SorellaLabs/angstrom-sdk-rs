mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

pub const POSITION_FETCHER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0x3D85e7B30BE9FD7A4bad709D6eD2d130579f9a2E");

pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0xEd421745765bc1938848cAaB502ffF53c653ff13");

pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0x293954613283cC7B82BfE9676D3cc0fb0A58fAa0");

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0xd99a23ECF12FD411660Be733caAe736777206011");

pub const POSITION_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0xF967Ede45ED04ec89EcA04a4c7175b6E0106e3A8");

pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0x48bC5A530873DcF0b890aD50120e7ee5283E0112");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
pub const POOL_CONFIG_STORE_SLOT: u8 = 3;

pub const BINANCE_REST_API_BASE_URL: &str = "https://api.binance.us";
