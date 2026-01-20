use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::{BlockNumber, Signature, U256, aliases::I24};
use alloy_provider::{
    Identity, Provider, RootProvider,
    fillers::{FillProvider, JoinFill, WalletFiller}
};
use alloy_signer::{Signer, SignerSync};
use angstrom_types_primitives::PoolId;
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::{WsClient, WsClientBuilder};
use uni_v4::pool_data_loader::TickData;

use crate::{
    l1::apis::node_api::{AngstromNodeApi, AngstromOrderApiClient},
    types::{pool_tick_loaders::PoolTickDataLoader, providers::AlloyProviderWrapper}
};

pub type AlloyWalletRpcProvider<P> =
    FillProvider<JoinFill<Identity, WalletFiller<EthereumWallet>>, AlloyProviderWrapper<P>>;

#[derive(Debug, Clone)]
pub struct AngstromProvider<P, T>
where
    P: Provider + Clone,
    T: AngstromOrderApiClient
{
    eth_provider:      AlloyProviderWrapper<P>,
    angstrom_provider: T
}

impl<P: Provider + Clone> AngstromProvider<P, HttpClient> {
    pub fn new_angstrom_http(eth_provider: P, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            eth_provider:      AlloyProviderWrapper::new(eth_provider),
            angstrom_provider: HttpClient::builder().build(angstrom_url)?
        })
    }
}

impl<P: Provider + Clone> AngstromProvider<P, WsClient> {
    pub async fn new_angstrom_ws(eth_provider: P, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            eth_provider:      AlloyProviderWrapper::new(eth_provider),
            angstrom_provider: WsClientBuilder::new().build(angstrom_url).await?
        })
    }
}

impl<P: Provider + Clone, T: AngstromOrderApiClient> AngstromProvider<P, T> {
    pub fn new_with_providers(eth_provider: P, angstrom_provider: T) -> Self {
        Self { eth_provider: AlloyProviderWrapper::new(eth_provider), angstrom_provider }
    }

    /// Returns the wrapped Ethereum provider.
    /// This wrapper implements both `Provider` and the SDK data APIs.
    pub fn eth_provider(&self) -> &AlloyProviderWrapper<P> {
        &self.eth_provider
    }

    pub fn with_wallet<S>(self, signer: S) -> AngstromProvider<AlloyWalletRpcProvider<P>, T>
    where
        S: Signer + SignerSync + TxSigner<Signature> + Send + Sync + 'static
    {
        let eth_provider = alloy_provider::builder::<Ethereum>()
            .wallet(EthereumWallet::new(signer))
            .connect_provider(self.eth_provider.clone());

        AngstromProvider {
            eth_provider:      AlloyProviderWrapper::new(eth_provider),
            angstrom_provider: self.angstrom_provider
        }
    }
}

impl<P: Provider + Clone, T: AngstromOrderApiClient> AngstromNodeApi<T> for AngstromProvider<P, T> {
    fn angstrom_rpc_provider(&self) -> &T {
        &self.angstrom_provider
    }
}

impl<P: Provider + Clone, T: AngstromOrderApiClient> Provider for AngstromProvider<P, T> {
    fn root(&self) -> &RootProvider {
        self.eth_provider.root()
    }
}

#[async_trait::async_trait]
impl<P, T> PoolTickDataLoader<Ethereum> for AngstromProvider<P, T>
where
    P: Provider + Clone + Sync,
    T: AngstromOrderApiClient + Sync
{
    async fn load_tick_data(
        &self,
        pool_id: PoolId,
        current_tick: I24,
        zero_for_one: bool,
        num_ticks: u16,
        tick_spacing: I24,
        block_number: Option<BlockNumber>
    ) -> eyre::Result<(Vec<TickData>, U256)> {
        self.eth_provider
            .load_tick_data(
                pool_id,
                current_tick,
                zero_for_one,
                num_ticks,
                tick_spacing,
                block_number
            )
            .await
    }
}
