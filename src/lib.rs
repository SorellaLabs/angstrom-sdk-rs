#![allow(async_fn_in_trait)]
#![allow(private_interfaces)]
#![allow(private_bounds)]

pub mod apis;
pub mod providers;
pub mod types;

use alloy_network::TxSigner;
use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_signer::{Signature, Signer, SignerSync};
use alloy_transport::Transport;
use providers::{AngstromProvider, EthRpcProvider, RpcWalletProvider};
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
    pub fn with_nonce_generator_filler(
        self,
        my_address: Address,
    ) -> AngstromApi<E, AngstromFillProvider<F, NonceGeneratorFiller>> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom: self.angstrom,
            config: self.config,
            filler: self
                .filler
                .wrap_with_filler(NonceGeneratorFiller::new(my_address)),
        }
    }

    pub fn with_token_balance_filler(
        self,
        my_address: Address,
    ) -> AngstromApi<E, AngstromFillProvider<F, TokenBalanceCheckFiller>> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom: self.angstrom,
            config: self.config,
            filler: self
                .filler
                .wrap_with_filler(TokenBalanceCheckFiller::new(my_address)),
        }
    }
}

impl<P, T, F> AngstromApi<EthRpcProvider<P, T>, F>
where
    P: Provider<T> + Clone,
    T: Transport + Clone,
    F: FillWrapper,
{
    pub fn with_signer_filler<S>(
        self,
        signer: S,
    ) -> AngstromApi<
        EthRpcProvider<RpcWalletProvider<P, T>, T>,
        AngstromFillProvider<F, SignerFiller<S>>,
    >
    where
        S: Signer + SignerSync + TxSigner<Signature> + Clone + Send + Sync + 'static,
        SignerFiller<S>: AngstromFiller,
    {
        AngstromApi {
            eth_provider: self.eth_provider.with_wallet(signer.clone()),
            angstrom: self.angstrom,
            config: self.config,
            filler: self.filler.wrap_with_filler(SignerFiller::new(signer)),
        }
    }
}

impl<E, F> AngstromApi<E, F>
where
    E: EthProvider,
    F: FillWrapper,
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
        S: Signer + SignerSync + Send,
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
