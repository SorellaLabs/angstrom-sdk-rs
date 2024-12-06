mod eth_rpc;
pub use eth_rpc::*;
mod angstrom;
pub use angstrom::*;
mod eth_provider;
pub use eth_provider::*;

use crate::providers::{AngstromProvider, EthProvider};

pub struct AngstromApi<E> {
    eth_provider: E,
    angstrom_provider: AngstromProvider,
}

impl<E: EthProvider> AngstromApi<E> {
    pub fn new(eth_provider: E, angstrom_provider: AngstromProvider) -> Self {
        Self {
            eth_provider,
            angstrom_provider,
        }
    }
}
