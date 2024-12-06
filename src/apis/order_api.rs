use alloy_primitives::{Address, FixedBytes, B256};
use angstrom_rpc::api::{CancelOrderRequest, GasEstimateResponse, OrderApiClient};
use angstrom_rpc::types::{
    OrderSubscriptionFilter, OrderSubscriptionKind, OrderSubscriptionResult,
};
use angstrom_types::orders::{OrderLocation, OrderStatus};
use jsonrpsee_http_client::HttpClient;
use std::collections::HashSet;
use std::future::Future;

use angstrom_types::sol_bindings::grouped_orders::AllOrders;
use futures::{Stream, StreamExt, TryStreamExt};

pub trait AngstromOrderApi {
    fn rpc_provider(&self) -> HttpClient;

    fn send_order(&self, order: AllOrders) -> impl Future<Output = eyre::Result<bool>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.send_order(order).await?) }
    }

    fn pending_order(
        &self,
        from: Address,
    ) -> impl Future<Output = eyre::Result<Vec<AllOrders>>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.pending_order(from).await?) }
    }

    fn cancel_order(
        &self,
        request: CancelOrderRequest,
    ) -> impl Future<Output = eyre::Result<bool>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.cancel_order(request).await?) }
    }

    fn estimate_gas(
        &self,
        order: AllOrders,
    ) -> impl Future<Output = eyre::Result<GasEstimateResponse>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.estimate_gas(order).await?) }
    }

    fn order_status(
        &self,
        order_hash: B256,
    ) -> impl Future<Output = eyre::Result<Option<OrderStatus>>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.order_status(order_hash).await?) }
    }

    fn orders_by_pair(
        &self,
        pair: FixedBytes<32>,
        location: OrderLocation,
    ) -> impl Future<Output = eyre::Result<Vec<AllOrders>>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.orders_by_pair(pair, location).await?) }
    }

    fn subscribe_orders(
        &self,
        kind: HashSet<OrderSubscriptionKind>,
        filters: HashSet<OrderSubscriptionFilter>,
    ) -> impl Future<
        Output = eyre::Result<impl Stream<Item = eyre::Result<OrderSubscriptionResult>> + Send>,
    > + Send {
        let provider = self.rpc_provider();
        async move {
            Ok(provider
                .subscribe_orders(kind, filters)
                .await?
                .map(|order| Ok(order?))
                .into_stream())
        }
    }

    fn send_orders(
        &self,
        orders: Vec<AllOrders>,
    ) -> impl Future<Output = eyre::Result<Vec<bool>>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.send_orders(orders).await?) }
    }

    fn pending_orders(
        &self,
        from: Vec<Address>,
    ) -> impl Future<Output = eyre::Result<Vec<AllOrders>>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.pending_orders(from).await?) }
    }

    fn cancel_orders(
        &self,
        request: Vec<CancelOrderRequest>,
    ) -> impl Future<Output = eyre::Result<Vec<bool>>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.cancel_orders(request).await?) }
    }

    fn estimate_gas_of_orders(
        &self,
        orders: Vec<AllOrders>,
    ) -> impl Future<Output = eyre::Result<Vec<GasEstimateResponse>>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.estimate_gas_of_orders(orders).await?) }
    }

    fn status_of_orders(
        &self,
        order_hashes: Vec<B256>,
    ) -> impl Future<Output = eyre::Result<Vec<Option<OrderStatus>>>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.status_of_orders(order_hashes).await?) }
    }

    fn orders_by_pairs(
        &self,
        pair_with_location: Vec<(FixedBytes<32>, OrderLocation)>,
    ) -> impl Future<Output = eyre::Result<Vec<AllOrders>>> + Send {
        let provider = self.rpc_provider();
        async move { Ok(provider.orders_by_pairs(pair_with_location).await?) }
    }
}
