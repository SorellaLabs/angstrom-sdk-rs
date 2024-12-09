#![allow(async_fn_in_trait)]
#![allow(private_interfaces)]

pub mod apis;
pub mod providers;
pub mod types;

use crate::providers::AngstromProvider;
use crate::providers::EthProvider;

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
