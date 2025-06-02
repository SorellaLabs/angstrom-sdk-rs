use alloy_network::TxSigner;
use alloy_primitives::{Address, FixedBytes, Signature};
use alloy_provider::Provider;
use alloy_signer::{Signer, SignerSync};
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey,
    contract_payloads::angstrom::{AngstromBundle, AngstromPoolConfigStore},
    sol_bindings::grouped_orders::AllOrders
};
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::WsClient;
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    apis::{
        data_api::AngstromDataApi,
        node_api::{AngstromNodeApi, AngstromOrderApiClient},
        user_api::AngstromUserApi
    },
    providers::backend::{AlloyWalletRpcProvider, AngstromProvider},
    types::{
        HistoricalOrders, HistoricalOrdersFilter, TokenInfoWithMeta, TokenPairInfo,
        UserLiquidityPosition,
        errors::AngstromSdkError,
        fillers::{
            AngstromFillProvider, AngstromFiller, AngstromSignerFiller, FillWrapper,
            NonceGeneratorFiller, TokenBalanceCheckFiller
        }
    }
};

#[derive(Clone)]
pub struct AngstromApi<P, T, F = ()>
where
    P: Provider,
    T: AngstromOrderApiClient
{
    provider: AngstromProvider<P, T>,
    filler:   F
}

impl<P: Provider> AngstromApi<P, HttpClient> {
    pub fn new_angstrom_http(eth_provider: P, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            provider: AngstromProvider::new_angstrom_http(eth_provider, angstrom_url)?,
            filler:   ()
        })
    }
}

impl<P: Provider> AngstromApi<P, WsClient> {
    pub async fn new_angstrom_ws(eth_provider: P, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            provider: AngstromProvider::new_angstrom_ws(eth_provider, angstrom_url).await?,
            filler:   ()
        })
    }
}

impl<P, T> AngstromApi<P, T>
where
    P: Provider,
    T: AngstromOrderApiClient
{
    pub(crate) fn new_with_provider(provider: AngstromProvider<P, T>) -> Self {
        Self { provider, filler: () }
    }
}

impl<P, T, F> AngstromApi<P, T, F>
where
    P: Provider,
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    pub fn eth_provider(&self) -> &P {
        self.provider.eth_provider()
    }

    pub fn angstrom_rpc_provider(&self) -> &T {
        self.provider.angstrom_rpc_provider()
    }

    pub fn angstrom_provider(&self) -> &AngstromProvider<P, T> {
        &self.provider
    }

    pub fn with_filler<F1: AngstromFiller>(
        self,
        filler: F1
    ) -> AngstromApi<P, T, AngstromFillProvider<F, F1>> {
        AngstromApi { provider: self.provider, filler: self.filler.wrap_with_filler(filler) }
    }

    pub fn with_nonce_generator_filler(
        self
    ) -> AngstromApi<P, T, AngstromFillProvider<F, NonceGeneratorFiller>> {
        AngstromApi {
            provider: self.provider,
            filler:   self.filler.wrap_with_filler(NonceGeneratorFiller)
        }
    }

    pub fn with_token_balance_filler(
        self
    ) -> AngstromApi<P, T, AngstromFillProvider<F, TokenBalanceCheckFiller>> {
        AngstromApi {
            provider: self.provider,
            filler:   self.filler.wrap_with_filler(TokenBalanceCheckFiller)
        }
    }

    pub fn with_angstrom_signer_filler<S>(
        self,
        signer: S
    ) -> AngstromApi<AlloyWalletRpcProvider<P>, T, AngstromFillProvider<F, AngstromSignerFiller<S>>>
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
        signer: S
    ) -> AngstromApi<
        P,
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
        AngstromSignerFiller<S>: FillWrapper,
        P: Provider
    {
        AngstromApi {
            provider: self.provider,
            filler:   self
                .filler
                .wrap_with_filler(NonceGeneratorFiller)
                .wrap_with_filler(TokenBalanceCheckFiller)
                .wrap_with_filler(AngstromSignerFiller::new(signer))
        }
    }

    pub fn from_address(&self) -> Option<Address> {
        self.filler.from()
    }
}

impl<P, T, F> AngstromNodeApi<T> for AngstromApi<P, T, F>
where
    P: Provider,
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

impl<P, T, F> AngstromDataApi for AngstromApi<P, T, F>
where
    P: Provider,
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        self.provider.all_token_pairs().await
    }

    async fn all_tokens(&self) -> eyre::Result<Vec<TokenInfoWithMeta>> {
        self.provider.all_tokens().await
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        self.provider
            .historical_orders(filter, block_stream_buffer)
            .await
    }

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        self.provider.pool_data(token0, token1, block_number).await
    }

    async fn historical_bundles(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<AngstromBundle>> {
        self.provider
            .historical_bundles(start_block, end_block, block_stream_buffer)
            .await
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        self.provider.pool_key(token0, token1).await
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>
    ) -> eyre::Result<AngstromPoolConfigStore> {
        self.provider.pool_config_store(block_number).await
    }
}

impl<P, T, F> AngstromUserApi for AngstromApi<P, T, F>
where
    P: Provider,
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    async fn get_positions(
        &self,
        user_address: Address
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        self.provider.get_positions(user_address).await
    }
}

#[cfg(test)]
impl<P, T, F> AngstromApi<P, T, F>
where
    P: Provider,
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    pub(crate) async fn fill(
        &self,
        order: &mut AllOrders
    ) -> Result<(), crate::types::fillers::errors::FillerError> {
        self.filler.fill(&self.provider, order).await
    }
}
