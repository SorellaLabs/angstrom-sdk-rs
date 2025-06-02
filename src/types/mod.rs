mod common;
use std::sync::OnceLock;

use alloy_dyn_abi::Eip712Domain;
use alloy_primitives::address;
pub use common::*;
mod historical_order_filters;
pub use historical_order_filters::*;

pub mod errors;
pub mod fillers;

use alloy_primitives::{Address, ChainId};

pub static ANGSTROM_ADDRESS: OnceLock<Address> = OnceLock::new();
pub static POSITION_MANAGER_ADDRESS: OnceLock<Address> = OnceLock::new();
pub static CONTROLLER_V1_ADDRESS: OnceLock<Address> = OnceLock::new();
pub static POOL_MANAGER_ADDRESS: OnceLock<Address> = OnceLock::new();
pub static ANGSTROM_DEPLOYED_BLOCK: OnceLock<u64> = OnceLock::new();
pub static ANGSTROM_DOMAIN: OnceLock<Eip712Domain> = OnceLock::new();

pub fn init_with_chain_id(chain_id: ChainId) {
    match chain_id {
        1 => {}
        11155111 => {
            ANGSTROM_ADDRESS
                .set(address!("0x9051085355BA7e36177e0a1c4082cb88C270ba90"))
                .unwrap();
            POSITION_MANAGER_ADDRESS
                .set(address!("0x429ba70129df741B2Ca2a85BC3A2a3328e5c09b4"))
                .unwrap();
            CONTROLLER_V1_ADDRESS
                .set(address!("0x73922Ee4f10a1D5A68700fF5c4Fbf6B0e5bbA674"))
                .unwrap();
            POOL_MANAGER_ADDRESS
                .set(address!("0xE03A1074c86CFeDd5C142C4F04F1a1536e203543"))
                .unwrap();
            ANGSTROM_DEPLOYED_BLOCK.set(8276506).unwrap();
            ANGSTROM_DOMAIN.set(alloy_sol_types::eip712_domain!(
                name: "Angstrom",
                version: "v1",
                chain_id: 11155111,
                verifying_contract: address!("0x9051085355BA7e36177e0a1c4082cb88C270ba90"),
            ));
        }
        id => panic!("unsupported chain_id: {}", id)
    }
}

// #[cfg(feature = "testnet-sepolia")]
// pub const POSITION_MANAGER_ADDRESS: alloy_primitives::Address =
//     angstrom_types::primitive::TESTNET_POSITION_MANAGER_ADDRESS;

// #[cfg(not(feature = "testnet-sepolia"))]
// pub const POSITION_MANAGER_ADDRESS: alloy_primitives::Address =
//     angstrom_types::primitive::POSITION_MANAGER_ADDRESS;
//
// #[cfg(feature = "testnet-sepolia")]
// pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
//     angstrom_types::primitive::TESTNET_CONTROLLER_V1_ADDRESS;
//
// #[cfg(not(feature = "testnet-sepolia"))]
// pub const CONTROLLER_V1_ADDRESS: alloy_primitives::Address =
//     angstrom_types::primitive::CONTROLLER_V1_ADDRESS;
//
// #[cfg(feature = "testnet-sepolia")]
// pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
//     angstrom_types::primitive::TESTNET_ANGSTROM_ADDRESS;
//
// #[cfg(not(feature = "testnet-sepolia"))]
// pub const ANGSTROM_ADDRESS: alloy_primitives::Address =
// angstrom_types::primitive::ANGSTROM_ADDRESS;
//
#[cfg(feature = "testnet-sepolia")]
pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
    angstrom_types::primitive::TESTNET_POOL_MANAGER_ADDRESS;

// #[cfg(not(feature = "testnet-sepolia"))]
// pub const POOL_MANAGER_ADDRESS: alloy_primitives::Address =
//     angstrom_types::primitive::POOL_MANAGER_ADDRESS;
//
// #[cfg(not(feature = "testnet-sepolia"))]
// pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 0;
// #[cfg(feature = "testnet-sepolia")]
// pub const ANGSTROM_DEPLOYED_BLOCK: u64 = 8276506;

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

pub const ANGSTROM_DOMAIN: alloy_sol_types::Eip712Domain = alloy_sol_types::eip712_domain!(
    name: "Angstrom",
    version: "v1",
    chain_id: 11155111,
    verifying_contract: ANGSTROM_ADDRESS,
);
