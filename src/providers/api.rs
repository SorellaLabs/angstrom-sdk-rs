use crate::apis::user_api::AngstromUserApi;
use crate::providers::backend::{AlloyWalletRpcProvider, AngstromProvider};
use crate::types::{
    HistoricalOrders, HistoricalOrdersFilter, TokenInfoWithMeta, TokenPairInfo,
    UserLiquidityPosition,
    errors::AngstromSdkError,
    fillers::{
        AngstromFillProvider, AngstromFiller, AngstromSignerFiller, FillWrapper,
        NonceGeneratorFiller, TokenBalanceCheckFiller,
    },
};
use alloy_network::TxSigner;
use alloy_primitives::{Address, FixedBytes, Signature};
use alloy_provider::{Provider, RootProvider};
use alloy_signer::{Signer, SignerSync};
use angstrom_types::contract_payloads::angstrom::AngstromPoolConfigStore;
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey, sol_bindings::grouped_orders::AllOrders,
};
use jsonrpsee_http_client::HttpClient;
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::apis::{data_api::AngstromDataApi, node_api::AngstromNodeApi};

use super::backend::AlloyRpcProvider;

#[derive(Clone)]
pub struct AngstromApi<P, F = ()>
where
    P: Provider,
{
    provider: AngstromProvider<P>,
    filler: F,
}

impl AngstromApi<AlloyRpcProvider<RootProvider>> {
    pub async fn new(eth_ws_url: &str, angstrom_http_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            provider: AngstromProvider::new(eth_ws_url, angstrom_http_url).await?,
            filler: (),
        })
    }
}

impl<P> AngstromApi<P>
where
    P: Provider,
{
    pub fn new_with_provider(provider: AngstromProvider<P>) -> Self {
        Self { provider, filler: () }
    }
}

impl<P, F> AngstromApi<P, F>
where
    P: Provider,
    F: FillWrapper,
{
    pub fn eth_provider(&self) -> &P {
        self.provider.eth_provider()
    }

    pub fn angstrom_rpc_provider(&self) -> HttpClient {
        self.provider.angstrom_rpc_provider()
    }

    pub fn angstrom_provider(&self) -> &AngstromProvider<P> {
        &self.provider
    }

    pub fn with_filler<F1: FillWrapper>(
        self,
        filler: F1,
    ) -> AngstromApi<P, AngstromFillProvider<F, F1>> {
        AngstromApi { provider: self.provider, filler: self.filler.wrap_with_filler(filler) }
    }

    pub fn with_nonce_generator_filler(
        self,
    ) -> AngstromApi<P, AngstromFillProvider<F, NonceGeneratorFiller>> {
        AngstromApi {
            provider: self.provider,
            filler: self.filler.wrap_with_filler(NonceGeneratorFiller),
        }
    }

    pub fn with_token_balance_filler(
        self,
    ) -> AngstromApi<P, AngstromFillProvider<F, TokenBalanceCheckFiller>> {
        AngstromApi {
            provider: self.provider,
            filler: self.filler.wrap_with_filler(TokenBalanceCheckFiller),
        }
    }

    pub fn with_angstrom_signer_filler<S>(
        self,
        signer: S,
    ) -> AngstromApi<AlloyWalletRpcProvider<P>, AngstromFillProvider<F, AngstromSignerFiller<S>>>
    where
        S: Signer + SignerSync + TxSigner<Signature> + Clone + Send + Sync + 'static,
        AngstromSignerFiller<S>: AngstromFiller,
    {
        AngstromApi {
            provider: self.provider.with_wallet(signer.clone()),
            filler: self
                .filler
                .wrap_with_filler(AngstromSignerFiller::new(signer)),
        }
    }

    pub fn with_all_fillers<S>(
        self,
        signer: S,
    ) -> AngstromApi<
        P,
        AngstromFillProvider<
            AngstromFillProvider<
                AngstromFillProvider<F, NonceGeneratorFiller>,
                TokenBalanceCheckFiller,
            >,
            AngstromSignerFiller<S>,
        >,
    >
    where
        S: Signer + SignerSync + Send + Clone,
        AngstromSignerFiller<S>: AngstromFiller,
        P: Provider,
    {
        AngstromApi {
            provider: self.provider,
            filler: self
                .filler
                .wrap_with_filler(NonceGeneratorFiller)
                .wrap_with_filler(TokenBalanceCheckFiller)
                .wrap_with_filler(AngstromSignerFiller::new(signer)),
        }
    }

    pub fn from_address(&self) -> Option<Address> {
        self.filler.from()
    }
}

impl<P, F> AngstromNodeApi for AngstromApi<P, F>
where
    P: Provider,
    F: FillWrapper,
{
    fn angstrom_rpc_provider(&self) -> HttpClient {
        self.provider.angstrom_rpc_provider()
    }

    async fn send_order(&self, mut order: AllOrders) -> Result<FixedBytes<32>, AngstromSdkError> {
        self.filler.fill(&self.provider, &mut order).await?;

        self.provider.send_order(order).await
    }

    async fn send_orders(
        &self,
        mut orders: Vec<AllOrders>,
    ) -> Result<Vec<Result<FixedBytes<32>, AngstromSdkError>>, AngstromSdkError> {
        self.filler.fill_many(&self.provider, &mut orders).await?;

        self.provider.send_orders(orders).await
    }
}

impl<P, F> AngstromDataApi for AngstromApi<P, F>
where
    P: Provider,
    F: FillWrapper,
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
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        self.provider.historical_orders(filter).await
    }

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        self.provider.pool_data(token0, token1, block_number).await
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        self.provider.pool_key(token0, token1).await
    }
    async fn pool_config_store(
        &self,
        block_number: Option<u64>,
    ) -> eyre::Result<AngstromPoolConfigStore> {
        self.provider.pool_config_store(block_number).await
    }
}

impl<P, F> AngstromUserApi for AngstromApi<P, F>
where
    P: Provider,
    F: FillWrapper,
{
    async fn get_positions(
        &self,
        user_address: Address,
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        self.provider.get_positions(user_address).await
    }
}

#[cfg(test)]
impl<P, F> AngstromApi<P, F>
where
    P: Provider,
    F: FillWrapper,
{
    pub(crate) async fn fill(
        &self,
        order: &mut AllOrders,
    ) -> Result<(), crate::types::fillers::errors::FillerError> {
        self.filler.fill(&self.provider, order).await
    }
}
