use alloy_primitives::{Address, FixedBytes, B256};
use angstrom_rpc::api::{GasEstimateResponse, OrderApiClient};
use angstrom_rpc::types::{
    OrderSubscriptionFilter, OrderSubscriptionKind, OrderSubscriptionResult,
};
use angstrom_types::orders::{CancelOrderRequest, OrderLocation, OrderStatus};
use jsonrpsee_http_client::HttpClient;
use std::collections::HashSet;

use angstrom_types::sol_bindings::grouped_orders::AllOrders;
use futures::{Stream, StreamExt, TryStreamExt};

pub trait AngstromNodeApi {
    fn rpc_provider(&self) -> HttpClient;

    async fn send_order(&self, order: AllOrders) -> eyre::Result<bool> {
        let provider = self.rpc_provider();
        Ok(provider.send_order(order).await?)
    }

    async fn pending_order(&self, from: Address) -> eyre::Result<Vec<AllOrders>> {
        let provider = self.rpc_provider();
        Ok(provider.pending_order(from).await?)
    }

    async fn cancel_order(&self, request: CancelOrderRequest) -> eyre::Result<bool> {
        let provider = self.rpc_provider();
        Ok(provider.cancel_order(request).await?)
    }

    async fn estimate_gas(&self, order: AllOrders) -> eyre::Result<GasEstimateResponse> {
        let provider = self.rpc_provider();
        Ok(provider.estimate_gas(order).await?)
    }

    async fn order_status(&self, order_hash: B256) -> eyre::Result<Option<OrderStatus>> {
        let provider = self.rpc_provider();
        Ok(provider.order_status(order_hash).await?)
    }

    async fn orders_by_pair(
        &self,
        pair: FixedBytes<32>,
        location: OrderLocation,
    ) -> eyre::Result<Vec<AllOrders>> {
        let provider = self.rpc_provider();
        Ok(provider.orders_by_pair(pair, location).await?)
    }

    async fn subscribe_orders(
        &self,
        kind: HashSet<OrderSubscriptionKind>,
        filters: HashSet<OrderSubscriptionFilter>,
    ) -> eyre::Result<impl Stream<Item = eyre::Result<OrderSubscriptionResult>>> {
        let provider = self.rpc_provider();

        Ok(provider
            .subscribe_orders(kind, filters)
            .await?
            .map(|order| Ok(order?))
            .into_stream())
    }

    async fn send_orders(&self, orders: Vec<AllOrders>) -> eyre::Result<Vec<bool>> {
        let provider = self.rpc_provider();
        Ok(provider.send_orders(orders).await?)
    }

    async fn pending_orders(&self, from: Vec<Address>) -> eyre::Result<Vec<AllOrders>> {
        let provider = self.rpc_provider();
        Ok(provider.pending_orders(from).await?)
    }

    async fn cancel_orders(&self, request: Vec<CancelOrderRequest>) -> eyre::Result<Vec<bool>> {
        let provider = self.rpc_provider();
        Ok(provider.cancel_orders(request).await?)
    }

    async fn estimate_gas_of_orders(
        &self,
        orders: Vec<AllOrders>,
    ) -> eyre::Result<Vec<GasEstimateResponse>> {
        let provider = self.rpc_provider();
        Ok(provider.estimate_gas_of_orders(orders).await?)
    }

    async fn status_of_orders(
        &self,
        order_hashes: Vec<B256>,
    ) -> eyre::Result<Vec<Option<OrderStatus>>> {
        let provider = self.rpc_provider();
        Ok(provider.status_of_orders(order_hashes).await?)
    }

    async fn orders_by_pairs(
        &self,
        pair_with_location: Vec<(FixedBytes<32>, OrderLocation)>,
    ) -> eyre::Result<Vec<AllOrders>> {
        let provider = self.rpc_provider();
        Ok(provider.orders_by_pairs(pair_with_location).await?)
    }
}

#[cfg(test)]
mod tests {

    use alloy_primitives::address;

    use crate::{
        apis::data_api::AngstromDataApi,
        test_utils::{make_generator, spawn_angstrom_provider, spawn_ws_provider},
    };

    use super::*;

    #[tokio::test]
    async fn test_pool_key() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let generator = make_generator(&eth_provider).await.unwrap();
        let binding = generator.generate_orders();
        let order = binding.first().unwrap();

        let tob_order = AllOrders::TOB(order.tob.clone());

        assert!(angstrom_provider.send_order(tob_order).await.unwrap());
    }
}
