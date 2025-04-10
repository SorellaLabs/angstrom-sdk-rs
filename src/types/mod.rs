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
pub const USDC: alloy_primitives::Address =
    alloy_primitives::address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
#[cfg(feature = "testnet-sepolia")]
pub const USDC: alloy_primitives::Address =
    alloy_primitives::address!("0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238");
