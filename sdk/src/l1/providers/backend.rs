use alloy_eips::BlockId;
use alloy_network::{Ethereum, EthereumWallet, Network, TxSigner};
use alloy_primitives::{Address, Signature, TxHash};
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_types::{Filter, Log};
use alloy_signer::{Signer, SignerSync};
use alloy_sol_types::{SolCall, SolType};
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::{WsClient, WsClientBuilder};

use crate::{
    l1::apis::node_api::{AngstromNodeApi, AngstromOrderApiClient},
    types::providers::{AlloyProviderWrapper, primitive_fetcher::PrimitivesFetcher}
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
    pub fn new_angstrom_http(
        eth_provider: impl Provider + 'static,
        angstrom_url: &str
    ) -> eyre::Result<Self> {
        Ok(Self {
            eth_provider:      AlloyProviderWrapper::new(eth_provider),
            angstrom_provider: HttpClient::builder().build(angstrom_url)?
        })
    }
}

impl AngstromProvider<WsClient> {
    pub async fn new_angstrom_ws(
        eth_provider: impl Provider + 'static,
        angstrom_url: &str
    ) -> eyre::Result<Self> {
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
impl<T> PrimitivesFetcher<Ethereum> for AngstromProvider<T>
where
    T: AngstromOrderApiClient + Sync
{
    async fn fetch_logs_primitive(&self, filter: &Filter) -> eyre::Result<Vec<Log>> {
        self.eth_provider.fetch_logs_primitive(filter).await
    }

    async fn view_call<IC>(
        &self,
        block_id: BlockId,
        contract: Address,
        call: IC
    ) -> eyre::Result<IC::Return>
    where
        IC: SolCall + Send
    {
        self.eth_provider.view_call(block_id, contract, call).await
    }

    async fn view_deploy_call<IC>(
        &self,
        block_id: BlockId,
        tx: <Ethereum as Network>::TransactionRequest
    ) -> eyre::Result<IC::RustType>
    where
        IC: SolType + Send
    {
        self.eth_provider.view_deploy_call::<IC>(block_id, tx).await
    }

    async fn alloy_root_provider(&self) -> eyre::Result<RootProvider<Ethereum>> {
        self.eth_provider.alloy_root_provider().await
    }

    async fn block_number_from_block_id(&self, block_id: BlockId) -> eyre::Result<u64> {
        self.eth_provider.block_number_from_block_id(block_id).await
    }

    async fn fetch_block_primitive(
        &self,
        block_id: BlockId,
        full: bool
    ) -> eyre::Result<<Ethereum as Network>::BlockResponse> {
        self.eth_provider
            .fetch_block_primitive(block_id, full)
            .await
    }

    async fn tx_success_primitive(&self, tx_hash: TxHash) -> eyre::Result<bool> {
        self.eth_provider.tx_success_primitive(tx_hash).await
    }

    async fn tx_by_hash_primitive(
        &self,
        tx_hash: TxHash
    ) -> eyre::Result<Option<<Ethereum as Network>::TransactionResponse>> {
        self.eth_provider.tx_by_hash_primitive(tx_hash).await
    }
}
