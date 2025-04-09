mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod fillers;

#[cfg(feature = "testnet-sepolia")]
pub const POSITION_MANAGER_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::TESTNET_POSITION_MANAGER_ADDRESS;

#[cfg(not(feature = "testnet-sepolia"))]
pub const POSITION_MANAGER_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::POSITION_MANAGER_ADDRESS;

#[cfg(feature = "testnet-sepolia")]
pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::TESTNET_CONTROLLER_V1_ADDRESS;

#[cfg(not(feature = "testnet-sepolia"))]
pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::CONTROLLER_V1_ADDRESS;

#[cfg(feature = "testnet-sepolia")]
pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::TESTNET_ANGSTROM_ADDRESS;

#[cfg(not(feature = "testnet-sepolia"))]
pub const ANGSTROM_ADDRESS: alloy_primitives::Address = angstrom_types::primitive::ANGSTROM_ADDRESS;

#[cfg(feature = "testnet-sepolia")]
pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::TESTNET_POOL_MANAGER_ADDRESS;

#[cfg(not(feature = "testnet-sepolia"))]
pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::POOL_MANAGER_ADDRESS;

pub const POOL_GATE_ADDRESS: alloy_primitives::Address =
    alloy_primitives::address!("0xd99a23ECF12FD411660Be733caAe736777206011");

pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
// pub const POOL_CONFIG_STORE_SLOT: u8 = 3;

#[cfg(not(feature = "testnet-sepolia"))]
pub(crate) const ANGSTROM_HTTP_URL: &str = "ANGSTROM_HTTP_URL";
#[cfg(feature = "testnet-sepolia")]
pub(crate) const ANGSTROM_HTTP_URL: &str = "ANGSTROM_SEPOLIA_HTTP_URL";
#[cfg(not(feature = "testnet-sepolia"))]
pub(crate) const ETH_WS_URL: &str = "ETH_WS_URL";
#[cfg(feature = "testnet-sepolia")]
pub(crate) const ETH_WS_URL: &str = "ETH_SEPOLIA_WS_URL";
