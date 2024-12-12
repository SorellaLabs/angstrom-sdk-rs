#![allow(async_fn_in_trait)]
#![allow(private_interfaces)]
#![allow(private_bounds)]

pub mod apis;
pub mod providers;
pub mod types;

use alloy_signer::{Signer, SignerSync};
use providers::AngstromProvider;
use types::fillers::{
    AngstromFillProvider, AngstromFiller, FillWrapper, NonceGeneratorFiller, SignerFiller,
    TokenBalanceCheckFiller,
};
use types::AngstromApiConfig;

use crate::providers::EthProvider;

pub struct AngstromApi<E, F> {
    eth_provider: E,
    angstrom: AngstromProvider,
    filler: F,
    config: AngstromApiConfig,
}

impl<E> AngstromApi<E, ()>
where
    E: EthProvider,
{
    pub fn new(eth_provider: E, angstrom: AngstromProvider, config: AngstromApiConfig) -> Self {
        Self {
            eth_provider,
            angstrom,
            config,
            filler: (),
        }
    }
}

impl<E, F> AngstromApi<E, F>
where
    E: EthProvider,
    F: FillWrapper,
{
    pub fn with_filler<OtherFiller: AngstromFiller>(
        self,
        filler: OtherFiller,
    ) -> AngstromApi<E, AngstromFillProvider<F, OtherFiller>> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom: self.angstrom,
            config: self.config,
            filler: self.filler.wrap_with_filler(filler),
        }
    }
}

impl<E, F> AngstromApi<E, F>
where
    E: EthProvider,
    F: FillWrapper,
    TokenBalanceCheckFiller: AngstromFiller,
{
    pub fn with_all_fillers<S>(
        self,
        signer: S,
    ) -> AngstromApi<
        E,
        AngstromFillProvider<
            AngstromFillProvider<
                AngstromFillProvider<F, NonceGeneratorFiller>,
                TokenBalanceCheckFiller,
            >,
            SignerFiller<S>,
        >,
    >
    where
        S: Signer + SignerSync,
        SignerFiller<S>: AngstromFiller,
    {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom: self.angstrom,
            config: self.config,
            filler: self
                .filler
                .wrap_with_filler(NonceGeneratorFiller::new(signer.address()))
                .wrap_with_filler(TokenBalanceCheckFiller::new(signer.address()))
                .wrap_with_filler(SignerFiller::new(signer)),
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
