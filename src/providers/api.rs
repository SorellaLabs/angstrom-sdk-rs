use crate::apis::{user_api::AngstromUserApi, utils::FromAddress};
use crate::providers::backend::{AlloyWalletRpcProvider, AngstromProvider};
use crate::types::{
    HistoricalOrders, HistoricalOrdersFilter, TokenInfoWithMeta, TokenPairInfo,
    TransactionRequestWithLiquidityMeta, UserLiquidityPosition,
    errors::AngstromSdkError,
    fillers::{
        AngstromFillProvider, AngstromFiller, AngstromSignerFiller, FillWrapper, FillerOrderFrom,
        MakeFillerOrder, NonceGeneratorFiller, TokenBalanceCheckFiller,
    },
};
use alloy_network::TxSigner;
use alloy_primitives::{Address, FixedBytes, PrimitiveSignature, TxHash};
use alloy_provider::{Provider, RootProvider};
use alloy_signer::{Signer, SignerSync};
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey,
    sol_bindings::{RawPoolOrder, grouped_orders::AllOrders},
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

    pub async fn send_add_remove_liquidity_tx(
        &self,
        tx_req: TransactionRequestWithLiquidityMeta,
    ) -> eyre::Result<TxHash> {
        let from = tx_req.from_address(&self.filler);
        let mut filled_tx_req = tx_req.convert_with_from(from);
        self.filler.fill(&self.provider, &mut filled_tx_req).await?;

        filled_tx_req.maybe_fill_modify_liquidity_call();

        Ok(self
            .provider
            .send_add_remove_liquidity_tx(filled_tx_req.inner.force_regular_tx())
            .await?)
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
        S: Signer + SignerSync + TxSigner<PrimitiveSignature> + Clone + Send + Sync + 'static,
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

    async fn send_order(&self, order: AllOrders) -> Result<FixedBytes<32>, AngstromSdkError> {
        let from = self.filler.from().unwrap_or_else(|| {
            let f = order.from();
            assert_ne!(f, Address::default());
            f
        });
        let mut filler_order: FillerOrderFrom = order.convert_with_from(from);
        self.filler.fill(&self.provider, &mut filler_order).await?;

        println!("{:?}", filler_order.inner);

        self.provider
            .send_order(filler_order.inner.force_angstrom_order())
            .await
    }

    async fn send_orders(
        &self,
        orders: Vec<AllOrders>,
    ) -> Result<Vec<Result<FixedBytes<32>, AngstromSdkError>>, AngstromSdkError> {
        let mut filler_orders: Vec<FillerOrderFrom> = orders
            .into_iter()
            .map(|o| {
                let from = o.from_address(&self.filler);
                o.convert_with_from(from)
            })
            .collect();

        self.filler
            .fill_many(&self.provider, &mut filler_orders)
            .await?;

        self.provider
            .send_orders(
                filler_orders
                    .into_iter()
                    .map(|order| order.inner.force_angstrom_order())
                    .collect(),
            )
            .await
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
        order: &mut FillerOrderFrom,
    ) -> Result<(), crate::types::fillers::errors::FillerError> {
        self.filler.fill(&self.provider, order).await
    }
}
