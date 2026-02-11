pub mod apis;
pub use providers::AngstromApi;

pub mod builders;
pub mod providers;
#[cfg(test)]
pub(crate) mod test_utils;

pub mod types;

use std::fmt::Debug;

use uniswap_storage::angstrom::mainnet::{
    ANGSTROM_L1_CONSTANTS_MAINNET, ANGSTROM_L1_CONSTANTS_SEPOLIA_TESTNET, AngstromL1Constants
};

#[derive(Debug, Clone, Copy)]
pub enum AngstromL1Chain {
    Mainnet,
    Sepolia
}

impl AngstromL1Chain {
    pub fn constants(&self) -> AngstromL1Constants {
        match self {
            AngstromL1Chain::Mainnet => ANGSTROM_L1_CONSTANTS_MAINNET,
            AngstromL1Chain::Sepolia => ANGSTROM_L1_CONSTANTS_SEPOLIA_TESTNET
        }
    }
}
