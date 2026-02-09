use alloy_network::{Ethereum, TxSigner};
use alloy_primitives::{Address, BlockNumber, FixedBytes, Signature, U256, aliases::I24};
use alloy_provider::Provider;
use alloy_signer::{Signer, SignerSync};
use angstrom_types_primitives::{PoolId, sol_bindings::grouped_orders::AllOrders};
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::WsClient;
use uni_v4::pool_data_loader::TickData;

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
    types::{pool_tick_loaders::PoolTickDataLoader, providers::AlloyProviderWrapper}
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
    pub fn new_angstrom_http(eth_provider: impl Provider + 'static, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            provider: AngstromProvider::new_angstrom_http(eth_provider, angstrom_url)?,
            filler:   ()
        })
    }
}

impl AngstromApi<WsClient> {
    pub async fn new_angstrom_ws(eth_provider: impl Provider + 'static, angstrom_url: &str) -> eyre::Result<Self> {
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
impl<T, F> PoolTickDataLoader<Ethereum> for AngstromApi<T, F>
where
    T: AngstromOrderApiClient + Sync,
    F: AngstromFiller + Sync
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
        self.provider
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
