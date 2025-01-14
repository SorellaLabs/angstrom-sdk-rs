#![allow(async_fn_in_trait)]
#![allow(private_interfaces)]
#![allow(private_bounds)]
#![feature(associated_type_defaults)]
#![feature(result_flattening)]

pub mod apis;
#[cfg(feature = "neon")]
pub mod js_utils;
pub mod providers;
#[cfg(test)]
pub mod test_utils;
pub mod types;

use alloy_network::TxSigner;
use alloy_primitives::{Address, PrimitiveSignature};
use alloy_provider::Provider;
use alloy_signer::{Signer, SignerSync};
use angstrom_rpc::api::GasEstimateResponse;
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey, primitive::PoolId,
    sol_bindings::grouped_orders::AllOrders
};
use apis::user_api::AngstromUserApi;
use jsonrpsee_http_client::HttpClient;
use providers::{AngstromProvider, EthRpcProvider, RpcWalletProvider};
use types::{
    fillers::{
        AngstromFillProvider, AngstromFiller, FillWrapper, FillerOrder, NonceGeneratorFiller,
        SignerFiller, TokenBalanceCheckFiller
    },
    HistoricalOrders, HistoricalOrdersFilter, TokenPairInfo
};
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};
use validation::order::OrderPoolNewOrderResult;

use crate::apis::{data_api::AngstromDataApi, node_api::AngstromNodeApi};

#[derive(Clone)]
pub struct AngstromApi<P, F = ()>
where
    P: Provider + Clone
{
    pub eth_provider: EthRpcProvider<P>,
    pub angstrom:     AngstromProvider,
    filler:           F
}

impl<P> AngstromApi<P>
where
    P: Provider + Clone
{
    pub fn new(eth_provider: EthRpcProvider<P>, angstrom: AngstromProvider) -> Self {
        Self { eth_provider, angstrom, filler: () }
    }
}

impl<P, F> AngstromApi<P, F>
where
    P: Provider + Clone,
    F: FillWrapper
{
    pub fn with_filler<F1: FillWrapper>(
        self,
        filler: F1
    ) -> AngstromApi<P, AngstromFillProvider<F, F1>> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom:     self.angstrom,
            filler:       self.filler.wrap_with_filler(filler)
        }
    }

    pub fn with_nonce_generator_filler(
        self
    ) -> AngstromApi<P, AngstromFillProvider<F, NonceGeneratorFiller>> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom:     self.angstrom,
            filler:       self.filler.wrap_with_filler(NonceGeneratorFiller)
        }
    }

    pub fn with_token_balance_filler(
        self
    ) -> AngstromApi<P, AngstromFillProvider<F, TokenBalanceCheckFiller>> {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom:     self.angstrom,
            filler:       self.filler.wrap_with_filler(TokenBalanceCheckFiller)
        }
    }

    pub fn with_signer_filler<S>(
        self,
        signer: S
    ) -> AngstromApi<RpcWalletProvider<P>, AngstromFillProvider<F, SignerFiller<S>>>
    where
        S: Signer + SignerSync + TxSigner<PrimitiveSignature> + Clone + Send + Sync + 'static,
        SignerFiller<S>: AngstromFiller
    {
        AngstromApi {
            eth_provider: self.eth_provider.with_wallet(signer.clone()),
            angstrom:     self.angstrom,
            filler:       self.filler.wrap_with_filler(SignerFiller::new(signer))
        }
    }

    pub fn with_all_fillers<S>(
        self,
        signer: S
    ) -> AngstromApi<
        P,
        AngstromFillProvider<
            AngstromFillProvider<
                AngstromFillProvider<F, NonceGeneratorFiller>,
                TokenBalanceCheckFiller
            >,
            SignerFiller<S>
        >
    >
    where
        S: Signer + SignerSync + Send,
        SignerFiller<S>: AngstromFiller,
        P: Provider + Clone
    {
        AngstromApi {
            eth_provider: self.eth_provider,
            angstrom:     self.angstrom,
            filler:       self
                .filler
                .wrap_with_filler(NonceGeneratorFiller)
                .wrap_with_filler(TokenBalanceCheckFiller)
                .wrap_with_filler(SignerFiller::new(signer))
        }
    }
}

impl<P, F> AngstromNodeApi for AngstromApi<P, F>
where
    P: Provider + Clone,
    F: FillWrapper
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
        orders: Vec<AllOrders>
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
                    .collect()
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
        orders: Vec<AllOrders>
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
                    .collect()
            )
            .await
    }
}

impl<P, F> AngstromDataApi for AngstromApi<P, F>
where
    P: Provider + Clone,
    F: FillWrapper
{
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        self.eth_provider.all_token_pairs().await
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        let orders = self.eth_provider.historical_orders(filter).await?;
        panic!("GOT HISTORICAL ORDERS: {orders:?}");
        Ok(orders)
    }

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<EnhancedUniswapPool<DataLoader<PoolId>, PoolId>> {
        self.eth_provider
            .pool_data(token0, token1, block_number)
            .await
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        self.eth_provider.pool_key(token0, token1).await
    }
}

impl<P, F> AngstromUserApi for AngstromApi<P, F>
where
    P: Provider + Clone,
    F: FillWrapper
{
    async fn get_positions(&self, user_address: Address) -> eyre::Result<()> {
        Ok(())
    }

    async fn get_pool_view(
        &self,
        user_address: Address,
        token0: Address,
        token1: Address
    ) -> eyre::Result<()> {
        Ok(())
    }
}
