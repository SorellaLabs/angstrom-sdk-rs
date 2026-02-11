use std::fmt::Debug;

use uniswap_storage::angstrom::l2::{
    ANGSTROM_L2_CONSTANTS_BASE_MAINNET, ANGSTROM_L2_CONSTANTS_UNICHAIN_MAINNET, AngstromL2Constants
};

pub mod apis;

#[cfg(test)]
pub(crate) mod test_utils;

#[derive(Debug, Clone, Copy)]
pub enum AngstromL2Chain {
    Base,
    Unichain
}

impl AngstromL2Chain {
    pub fn constants(&self) -> AngstromL2Constants {
        match self {
            AngstromL2Chain::Base => ANGSTROM_L2_CONSTANTS_BASE_MAINNET,
            AngstromL2Chain::Unichain => ANGSTROM_L2_CONSTANTS_UNICHAIN_MAINNET
        }
    }
}
