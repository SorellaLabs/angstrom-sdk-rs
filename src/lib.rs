#![allow(async_fn_in_trait)]
#![allow(private_interfaces)]
#![allow(private_bounds)]

pub mod apis;
pub mod providers;
#[cfg(test)]
pub mod test_utils;
pub mod types;

use crate::apis::data_api::AngstromDataApi;
use crate::apis::node_api::AngstromNodeApi;
use alloy_network::TxSigner;
use alloy_primitives::Address;
use alloy_primitives::PrimitiveSignature;
use alloy_provider::Provider;
use alloy_signer::{Signer, SignerSync};
use alloy_transport::Transport;
use angstrom_rpc::api::GasEstimateResponse;
use angstrom_types::contract_bindings::angstrom::Angstrom::PoolKey;
use angstrom_types::primitive::PoolId;
use angstrom_types::sol_bindings::grouped_orders::AllOrders;
use apis::order_builder::AngstromOrderBuilder;
use jsonrpsee_http_client::HttpClient;
use providers::{AngstromProvider, EthRpcProvider, RpcWalletProvider};
use types::fillers::FillerOrder;
use types::fillers::{
    AngstromFillProvider, AngstromFiller, FillWrapper, NonceGeneratorFiller, SignerFiller,
    TokenBalanceCheckFiller,
};

use types::HistoricalOrders;
use types::HistoricalOrdersFilter;
use types::TokenPairInfo;
use uniswap_v4::uniswap::pool::EnhancedUniswapPool;
use uniswap_v4::uniswap::pool_data_loader::DataLoader;
use validation::order::OrderPoolNewOrderResult;

pub struct AngstromApi<P, T, F = ()>
where
    P: Provider<T> + Clone,
    T: Transport + Clone,
{
    pub eth_provider: EthRpcProvider<P, T>,
    pub angstrom: AngstromProvider,
    filler: F,
}

impl<P, T> AngstromApi<P, T>
where
    P: Provider<T> + Clone,
    T: Transport + Clone,
{
    pub fn new(eth_provider: EthRpcProvider<P, T>, angstrom: AngstromProvider) -> Self {
        Self {
            eth_provider,
            angstrom,
            filler: (),
        }
    }
}

impl<P, T, F> AngstromApi<P, T, F>
where
    P: Provider<T> + Clone,
    T: Transport + Clone,
    F: FillWrapper,
{
    pub fn with_nonce_generator_filler(
        self,
    ) -> AngstromApi<P, T, AngstromFillProvider<F, NonceGeneratorFiller>> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom: self.angstrom,
            filler: self.filler.wrap_with_filler(NonceGeneratorFiller),
        }
    }

    pub fn with_token_balance_filler(
        self,
    ) -> AngstromApi<P, T, AngstromFillProvider<F, TokenBalanceCheckFiller>> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom: self.angstrom,
            filler: self.filler.wrap_with_filler(TokenBalanceCheckFiller),
        }
    }

    pub fn with_signer_filler<S>(
        self,
        signer: S,
    ) -> AngstromApi<RpcWalletProvider<P, T>, T, AngstromFillProvider<F, SignerFiller<S>>>
    where
        S: Signer + SignerSync + TxSigner<PrimitiveSignature> + Clone + Send + Sync + 'static,
        SignerFiller<S>: AngstromFiller,
    {
        AngstromApi {
            eth_provider: self.eth_provider.with_wallet(signer.clone()),
            angstrom: self.angstrom,
            filler: self.filler.wrap_with_filler(SignerFiller::new(signer)),
        }
    }

    pub fn with_all_fillers<S>(
        self,
        signer: S,
    ) -> AngstromApi<
        P,
        T,
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
        P: Provider<T> + Clone,
        T: Transport + Clone,
    {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom: self.angstrom,
            filler: self
                .filler
                .wrap_with_filler(NonceGeneratorFiller)
                .wrap_with_filler(TokenBalanceCheckFiller)
                .wrap_with_filler(SignerFiller::new(signer)),
        }
    }
}

impl<P, T, F> AngstromNodeApi for AngstromApi<P, T, F>
where
    P: Provider<T> + Clone,
    T: Transport + Clone,
    F: FillWrapper,
{
    fn rpc_provider(&self) -> HttpClient {
        self.angstrom.rpc_provider()
    }

    async fn send_order(&self, order: AllOrders) -> eyre::Result<OrderPoolNewOrderResult> {
        let mut filler_order: FillerOrder = order.into();
        self.filler
            .fill(&self.eth_provider, &self.angstrom, &mut filler_order)
            .await?;
        self.angstrom
            .send_order(filler_order.force_all_orders())
            .await
    }

    async fn send_orders(
        &self,
        orders: Vec<AllOrders>,
    ) -> eyre::Result<Vec<OrderPoolNewOrderResult>> {
        let mut filler_orders: Vec<FillerOrder> = orders.into_iter().map(Into::into).collect();

        self.filler
            .fill_many(&self.eth_provider, &self.angstrom, &mut filler_orders)
            .await?;
        self.angstrom
            .send_orders(
                filler_orders
                    .into_iter()
                    .map(|order| order.force_all_orders())
                    .collect(),
            )
            .await
    }

    async fn estimate_gas(&self, order: AllOrders) -> eyre::Result<GasEstimateResponse> {
        let mut filler_order: FillerOrder = order.into();
        self.filler
            .fill(&self.eth_provider, &self.angstrom, &mut filler_order)
            .await?;

        self.angstrom
            .estimate_gas(filler_order.force_all_orders())
            .await
    }

    async fn estimate_gas_of_orders(
        &self,
        orders: Vec<AllOrders>,
    ) -> eyre::Result<Vec<GasEstimateResponse>> {
        let mut filler_orders: Vec<FillerOrder> = orders.into_iter().map(Into::into).collect();

        self.filler
            .fill_many(&self.eth_provider, &self.angstrom, &mut filler_orders)
            .await?;
        self.angstrom
            .estimate_gas_of_orders(
                filler_orders
                    .into_iter()
                    .map(|order| order.force_all_orders())
                    .collect(),
            )
            .await
    }
}

impl<P, T, F> AngstromDataApi for AngstromApi<P, T, F>
where
    P: Provider<T> + Clone,
    T: Transport + Clone,
    F: FillWrapper,
{
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        self.eth_provider.all_token_pairs().await
    }

    async fn historical_orders(
        &self,
        filter: &HistoricalOrdersFilter,
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        self.eth_provider.historical_orders(filter).await
    }

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<EnhancedUniswapPool<DataLoader<PoolId>, PoolId>> {
        self.eth_provider
            .pool_data(token0, token1, block_number)
            .await
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        self.eth_provider.pool_key(token0, token1).await
    }
}

impl<P, T, F> AngstromOrderBuilder for AngstromApi<P, T, F>
where
    P: Provider<T> + Clone,
    T: Transport + Clone,
    F: FillWrapper,
{
}
