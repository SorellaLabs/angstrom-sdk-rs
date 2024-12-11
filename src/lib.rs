#![allow(async_fn_in_trait)]
#![allow(private_interfaces)]
#![allow(private_bounds)]

pub mod apis;
pub mod providers;
pub mod types;

use std::marker::PhantomData;

use alloy_primitives::Address;
use providers::{AngstromFillProvider, AngstromProvider};
use types::fillers::{AngstromFiller, FillWrapper, TokenBalanceCheckFiller};
use types::AngstromApiConfig;

use crate::providers::EthProvider;

pub struct AngstromApi<E, F, O> {
    eth_provider: E,
    angstrom: AngstromProvider,
    filler: F,
    config: AngstromApiConfig,
    _phantom: PhantomData<O>,
}

impl<E, O> AngstromApi<E, (), O>
where
    E: EthProvider,
{
    pub fn new(eth_provider: E, angstrom: AngstromProvider, config: AngstromApiConfig) -> Self {
        Self {
            eth_provider,
            angstrom,
            config,
            filler: (),
            _phantom: PhantomData,
        }
    }
}

impl<E, F, O> AngstromApi<E, F, O>
where
    E: EthProvider,
    F: FillWrapper<O>,
{
    pub fn with_filler<OtherFiller: AngstromFiller<O>>(
        self,
        filler: OtherFiller,
    ) -> AngstromApi<E, OtherFiller, O> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom: self.angstrom,
            config: self.config,
            filler,
            _phantom: self._phantom,
        }
    }
}

impl<E, F, O> AngstromApi<E, F, O>
where
    E: EthProvider,
    F: FillWrapper<O>,
    TokenBalanceCheckFiller: AngstromFiller<O>,
{
    pub fn with_token_balance_check_filter(
        self,
        my_address: Address,
    ) -> AngstromApi<E, AngstromFillProvider<F, TokenBalanceCheckFiller>, O> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom: self.angstrom,
            config: self.config,
            filler: self
                .filler
                .wrap_with_filler(TokenBalanceCheckFiller::new(my_address)),
            _phantom: self._phantom,
        }
    }
}

impl<E, F, O> AngstromApi<E, F, O>
where
    E: EthProvider,
    F: FillWrapper<O>,
    TokenBalanceCheckFiller: AngstromFiller<O>,
{
    pub fn with_all_fillers(
        self,
        my_address: Address,
    ) -> AngstromApi<E, AngstromFillProvider<F, TokenBalanceCheckFiller>, O> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom: self.angstrom,
            config: self.config,
            filler: self
                .filler
                .wrap_with_filler(TokenBalanceCheckFiller::new(my_address)),
            _phantom: self._phantom,
        }
    }
}

#[cfg(test)]
mod tests {
    // use providers::{AngstromFillProvider, EthRpcProvider};
    // use types::fillers::TokenBalanceCheckFiller;

    // use super::*;

    // async fn init_angstrom_api<R, O>() -> eyre::Result<()> {
    //     let angstrom = AngstromProvider {};

    //     let filler = AngstromFillProvider::new((), ());
    //     let config = AngstromApiConfig::new("35.245.117.24".to_owned(), "", 0, 8546);

    //     let eth_provider = EthRpcProvider::new_ws(config.ws_url()).await?;

    //     Ok(())
    // }
}
