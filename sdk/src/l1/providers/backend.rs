use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::{BlockNumber, Signature, U256, aliases::I24};
use alloy_provider::{Provider, RootProvider};
use alloy_signer::{Signer, SignerSync};
use angstrom_types_primitives::PoolId;
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::{WsClient, WsClientBuilder};
use uni_v4::pool_data_loader::TickData;

use crate::{
    l1::apis::node_api::{AngstromNodeApi, AngstromOrderApiClient},
    types::{pool_tick_loaders::PoolTickDataLoader, providers::AlloyProviderWrapper}
};

#[derive(Debug, Clone)]
pub struct AngstromProvider<T>
where
    T: AngstromOrderApiClient
{
    eth_provider:      AlloyProviderWrapper,
    angstrom_provider: T
}

impl AngstromProvider<HttpClient> {
    pub fn new_angstrom_http(eth_provider: impl Provider + 'static, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            eth_provider:      AlloyProviderWrapper::new(eth_provider),
            angstrom_provider: HttpClient::builder().build(angstrom_url)?
        })
    }
}

impl AngstromProvider<WsClient> {
    pub async fn new_angstrom_ws(eth_provider: impl Provider + 'static, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            eth_provider:      AlloyProviderWrapper::new(eth_provider),
            angstrom_provider: WsClientBuilder::new().build(angstrom_url).await?
        })
    }
}

impl<T: AngstromOrderApiClient> AngstromProvider<T> {
    pub fn new_with_providers(eth_provider: impl Provider + 'static, angstrom_provider: T) -> Self {
        Self { eth_provider: AlloyProviderWrapper::new(eth_provider), angstrom_provider }
    }

    /// Returns the wrapped Ethereum provider.
    /// This wrapper implements both `Provider` and the SDK data APIs.
    pub fn eth_provider(&self) -> &AlloyProviderWrapper {
        &self.eth_provider
    }

    pub fn with_wallet<S>(self, signer: S) -> AngstromProvider<T>
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

impl<T: AngstromOrderApiClient> AngstromNodeApi<T> for AngstromProvider<T> {
    fn angstrom_rpc_provider(&self) -> &T {
        &self.angstrom_provider
    }
}

impl<T: AngstromOrderApiClient> Provider for AngstromProvider<T> {
    fn root(&self) -> &RootProvider {
        self.eth_provider.root()
    }
}

#[async_trait::async_trait]
impl<T> PoolTickDataLoader<Ethereum> for AngstromProvider<T>
where
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
