use std::collections::HashSet;

use alloy_primitives::{Address, B256, FixedBytes, TxHash, U256};
use angstrom_rpc_api::OrderApiClient;
use angstrom_rpc_types::{
    OrderSubscriptionFilter, OrderSubscriptionKind, OrderSubscriptionResult, PendingOrder
};
use angstrom_types_primitives::{
    orders::CancelOrderRequest,
    primitive::{OrderLocation, OrderStatus, PoolId},
    sol_bindings::grouped_orders::AllOrders
};
use auto_impl::auto_impl;
use futures::{Stream, StreamExt, TryStreamExt};
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::WsClient;

use crate::l1::types::errors::AngstromSdkError;

#[auto_impl(&, Box, Arc)]
pub trait AngstromOrderApiClient: OrderApiClient + Send + Sync {}
impl AngstromOrderApiClient for WsClient {}
impl AngstromOrderApiClient for HttpClient {}

#[async_trait::async_trait]
#[auto_impl(&, Box, Arc)]
pub trait AngstromNodeApi<T: AngstromOrderApiClient>: Send + Sync {
    fn angstrom_rpc_provider(&self) -> &T;

    async fn send_order(&self, order: AllOrders) -> Result<TxHash, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        let result = provider.send_order(order).await?;

        let out = if result.is_success {
            Ok(serde_json::from_value(result.data).unwrap())
        } else {
            Err(result.msg)
        };

        out.map_err(AngstromSdkError::AngstromRpc)
    }

    async fn pending_order(&self, from: Address) -> Result<Vec<PendingOrder>, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        Ok(provider.pending_order(from).await?)
    }

    async fn cancel_order(&self, request: CancelOrderRequest) -> Result<bool, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        Ok(provider.cancel_order(request).await?)
    }

    async fn estimate_angstrom_gas(
        &self,
        is_book: bool,
        is_internal: bool,
        token_0: Address,
        token_1: Address
    ) -> Result<(U256, u64), AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        provider
            .estimate_gas(is_book, is_internal, token_0, token_1)
            .await?
            .map_err(AngstromSdkError::AngstromRpc)
    }

    async fn order_status(&self, order_hash: B256) -> Result<OrderStatus, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        Ok(serde_json::from_value(provider.order_status(order_hash).await?.data).unwrap())
    }

    async fn orders_by_pool_id(
        &self,
        pool_id: PoolId,
        location: OrderLocation
    ) -> Result<Vec<AllOrders>, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        Ok(provider.orders_by_pool_id(pool_id, location).await?)
    }

    async fn subscribe_orders(
        &self,
        kind: HashSet<OrderSubscriptionKind>,
        filters: HashSet<OrderSubscriptionFilter>
    ) -> Result<
        impl Stream<Item = Result<OrderSubscriptionResult, AngstromSdkError>>,
        AngstromSdkError
    > {
        let provider = self.angstrom_rpc_provider();

        Ok(provider
            .subscribe_orders(kind, filters)
            .await?
            .map(|order| order.map_err(|e| AngstromSdkError::AngstromRpc(e.to_string())))
            .into_stream())
    }

    async fn send_orders(
        &self,
        orders: Vec<AllOrders>
    ) -> Result<Vec<Result<FixedBytes<32>, AngstromSdkError>>, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        Ok(provider
            .send_orders(orders)
            .await?
            .into_iter()
            .map(|result| {
                let out = if result.is_success {
                    Ok(serde_json::from_value(result.data).unwrap())
                } else {
                    Err(result.msg)
                };

                out.map_err(AngstromSdkError::AngstromRpc)
            })
            .collect())
    }

    async fn pending_orders(
        &self,
        from: Vec<Address>
    ) -> Result<Vec<PendingOrder>, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        Ok(provider.pending_orders(from).await?)
    }

    async fn cancel_orders(
        &self,
        request: Vec<CancelOrderRequest>
    ) -> Result<Vec<bool>, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        Ok(provider.cancel_orders(request).await?)
    }

    async fn estimate_gas_of_orders(
        &self,
        orders: Vec<(bool, bool, Address, Address)>
    ) -> Result<Vec<Result<(U256, u64), AngstromSdkError>>, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        Ok(provider
            .estimate_gas_of_orders(orders)
            .await?
            .into_iter()
            .map(|r| r.map_err(AngstromSdkError::AngstromRpc))
            .collect())
    }

    async fn status_of_orders(
        &self,
        order_hashes: Vec<B256>
    ) -> Result<Vec<OrderStatus>, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        Ok(provider
            .status_of_orders(order_hashes)
            .await?
            .into_iter()
            .map(|s| serde_json::from_value(s.data).unwrap())
            .collect())
    }

    async fn orders_by_pool_ids(
        &self,
        pool_ids_with_location: Vec<(PoolId, OrderLocation)>
    ) -> Result<Vec<AllOrders>, AngstromSdkError> {
        let provider = self.angstrom_rpc_provider();
        Ok(provider.orders_by_pool_ids(pool_ids_with_location).await?)
    }
}
