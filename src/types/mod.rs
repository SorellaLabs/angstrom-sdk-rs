mod common;
pub use common::*;

mod historical_order_filters;
pub use historical_order_filters::*;

pub mod errors;
pub mod fillers;

pub use angstrom_types::primitive::ANGSTROM_DOMAIN;

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

#[cfg(not(feature = "testnet-sepolia"))]
pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
#[cfg(feature = "testnet-sepolia")]
pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;

#[cfg(not(feature = "testnet-sepolia"))]
pub const USDC: alloy_primitives::Address =
    alloy_primitives::address!("0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
#[cfg(feature = "testnet-sepolia")]
pub const USDC: alloy_primitives::Address =
    alloy_primitives::address!("0x1c7D4B196Cb0C7B01d743Fbc6116a902379C7238");

#[cfg(not(feature = "testnet-sepolia"))]
pub const WETH: alloy_primitives::Address =
    alloy_primitives::address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
#[cfg(feature = "testnet-sepolia")]
pub const WETH: alloy_primitives::Address =
    alloy_primitives::address!("0xfFf9976782d46CC05630D1f6eBAb18b2324d6B14");

#[cfg(not(feature = "testnet-sepolia"))]
pub const UNI: alloy_primitives::Address =
    alloy_primitives::address!("0x1f9840a85d5af5bf1d1762f925bdaddc4201f984");
#[cfg(feature = "testnet-sepolia")]
pub const UNI: alloy_primitives::Address =
    alloy_primitives::address!("0x1f9840a85d5aF5bf1D1762F925BDADdC4201F984");
