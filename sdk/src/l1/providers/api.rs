use alloy_eips::BlockId;
use alloy_network::{Ethereum, Network, TxSigner};
use alloy_primitives::{Address, FixedBytes, Signature, TxHash};
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_types::{Filter, Log};
use alloy_signer::{Signer, SignerSync};
use alloy_sol_types::{SolCall, SolType};
use angstrom_types_primitives::sol_bindings::grouped_orders::AllOrders;
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::WsClient;

use crate::{
    l1::{
        AngstromL1Chain,
        apis::node_api::{AngstromNodeApi, AngstromOrderApiClient},
        providers::backend::AngstromProvider,
        types::{
            errors::AngstromSdkError,
            fillers::{
                AngstromFillProvider, AngstromFiller, AngstromSignerFiller, FillWrapper,
                NonceGeneratorFiller, TokenBalanceCheckFiller
            }
        }
    },
    types::providers::{AlloyProviderWrapper, primitive_fetcher::PrimitivesFetcher}
};

#[derive(Clone)]
pub struct AngstromApi<T, F = ()>
where
    T: AngstromOrderApiClient
{
    provider: AngstromProvider<T>,
    filler:   F
}

impl AngstromApi<HttpClient> {
    pub fn new_angstrom_http(
        eth_provider: impl Provider + 'static,
        angstrom_url: &str
    ) -> eyre::Result<Self> {
        Ok(Self {
            provider: AngstromProvider::new_angstrom_http(eth_provider, angstrom_url)?,
            filler:   ()
        })
    }
}

impl AngstromApi<WsClient> {
    pub async fn new_angstrom_ws(
        eth_provider: impl Provider + 'static,
        angstrom_url: &str
    ) -> eyre::Result<Self> {
        Ok(Self {
            provider: AngstromProvider::new_angstrom_ws(eth_provider, angstrom_url).await?,
            filler:   ()
        })
    }
}

impl<T> AngstromApi<T>
where
    T: AngstromOrderApiClient
{
    #[allow(unused)]
    pub fn new_with_provider(provider: AngstromProvider<T>) -> Self {
        Self { provider, filler: () }
    }

    pub fn new_with_providers(eth: impl Provider + 'static, ang: T) -> Self {
        Self { provider: AngstromProvider::new_with_providers(eth, ang), filler: () }
    }
}

impl<T, F> AngstromApi<T, F>
where
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    /// Returns the wrapped Ethereum provider.
    /// This wrapper implements both `Provider` and the SDK data APIs.
    pub fn eth_provider(&self) -> &AlloyProviderWrapper {
        self.provider.eth_provider()
    }

    pub fn angstrom_rpc_provider(&self) -> &T {
        self.provider.angstrom_rpc_provider()
    }

    pub fn angstrom_provider(&self) -> &AngstromProvider<T> {
        &self.provider
    }

    pub fn with_filler<F1: AngstromFiller>(
        self,
        filler: F1
    ) -> AngstromApi<T, AngstromFillProvider<F, F1>> {
        AngstromApi { provider: self.provider, filler: self.filler.wrap_with_filler(filler) }
    }

    pub fn with_nonce_generator_filler(
        self,
        chain: AngstromL1Chain
    ) -> AngstromApi<T, AngstromFillProvider<F, NonceGeneratorFiller>> {
        AngstromApi {
            provider: self.provider,
            filler:   self.filler.wrap_with_filler(NonceGeneratorFiller(chain))
        }
    }

    pub fn with_token_balance_filler(
        self
    ) -> AngstromApi<T, AngstromFillProvider<F, TokenBalanceCheckFiller>> {
        AngstromApi {
            provider: self.provider,
            filler:   self.filler.wrap_with_filler(TokenBalanceCheckFiller)
        }
    }

    pub fn with_angstrom_signer_filler<S>(
        self,
        signer: S
    ) -> AngstromApi<T, AngstromFillProvider<F, AngstromSignerFiller<S>>>
    where
        S: Signer + SignerSync + TxSigner<Signature> + Clone + Send + Sync + 'static,
        AngstromSignerFiller<S>: FillWrapper
    {
        AngstromApi {
            provider: self.provider.with_wallet(signer.clone()),
            filler:   self
                .filler
                .wrap_with_filler(AngstromSignerFiller::new(signer))
        }
    }

    pub fn with_all_fillers<S>(
        self,
        signer: S,
        chain: AngstromL1Chain
    ) -> AngstromApi<
        T,
        AngstromFillProvider<
            AngstromFillProvider<
                AngstromFillProvider<F, NonceGeneratorFiller>,
                TokenBalanceCheckFiller
            >,
            AngstromSignerFiller<S>
        >
    >
    where
        S: Signer + SignerSync + Send + Clone,
        AngstromSignerFiller<S>: FillWrapper
    {
        AngstromApi {
            provider: self.provider,
            filler:   self
                .filler
                .wrap_with_filler(NonceGeneratorFiller(chain))
                .wrap_with_filler(TokenBalanceCheckFiller)
                .wrap_with_filler(AngstromSignerFiller::new(signer))
        }
    }

    pub fn from_address(&self) -> Option<Address> {
        self.filler.from()
    }
}

#[async_trait::async_trait]
impl<T, F> AngstromNodeApi<T> for AngstromApi<T, F>
where
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    fn angstrom_rpc_provider(&self) -> &T {
        self.provider.angstrom_rpc_provider()
    }

    async fn send_order(&self, mut order: AllOrders) -> Result<FixedBytes<32>, AngstromSdkError> {
        self.filler.fill(&self.provider, &mut order).await?;

        self.provider.send_order(order).await
    }

    async fn send_orders(
        &self,
        mut orders: Vec<AllOrders>
    ) -> Result<Vec<Result<FixedBytes<32>, AngstromSdkError>>, AngstromSdkError> {
        self.filler.fill_many(&self.provider, &mut orders).await?;

        self.provider.send_orders(orders).await
    }
}

impl<T, F> Provider for AngstromApi<T, F>
where
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    fn root(&self) -> &alloy_provider::RootProvider<alloy_network::Ethereum> {
        self.eth_provider().root()
    }
}

#[async_trait::async_trait]
impl<T, F> PrimitivesFetcher<Ethereum> for AngstromApi<T, F>
where
    T: AngstromOrderApiClient + Sync,
    F: AngstromFiller + Sync
{
    async fn fetch_logs_primitive(&self, filter: &Filter) -> eyre::Result<Vec<Log>> {
        self.provider.fetch_logs_primitive(filter).await
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
        self.provider.view_call(block_id, contract, call).await
    }

    async fn view_deploy_call<IC>(
        &self,
        block_id: BlockId,
        tx: <Ethereum as Network>::TransactionRequest
    ) -> eyre::Result<IC::RustType>
    where
        IC: SolType + Send
    {
        self.provider.view_deploy_call::<IC>(block_id, tx).await
    }

    async fn alloy_root_provider(&self) -> eyre::Result<RootProvider<Ethereum>> {
        self.provider.alloy_root_provider().await
    }

    async fn block_number_from_block_id(&self, block_id: BlockId) -> eyre::Result<u64> {
        self.provider.block_number_from_block_id(block_id).await
    }

    async fn fetch_block_primitive(
        &self,
        block_id: BlockId,
        full: bool
    ) -> eyre::Result<<Ethereum as Network>::BlockResponse> {
        self.provider.fetch_block_primitive(block_id, full).await
    }

    async fn tx_success(&self, tx_hash: TxHash) -> eyre::Result<bool> {
        self.provider.tx_success(tx_hash).await
    }

    async fn tx_by_hash_primitive(
        &self,
        tx_hash: TxHash
    ) -> eyre::Result<Option<<Ethereum as Network>::TransactionResponse>> {
        self.provider.tx_by_hash_primitive(tx_hash).await
    }
}

#[cfg(test)]
impl<T, F> AngstromApi<T, F>
where
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    pub(crate) async fn fill(
        &self,
        order: &mut AllOrders
    ) -> Result<(), crate::l1::types::fillers::errors::FillerError> {
        self.filler.fill(&self.provider, order).await
    }
}
