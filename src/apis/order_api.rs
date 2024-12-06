use alloy_primitives::map::HashSet;
use alloy_primitives::{Address, FixedBytes, B256};
use angstrom_rpc::api::{CancelOrderRequest, GasEstimateResponse};
use angstrom_rpc::types::{OrderSubscriptionFilter, OrderSubscriptionKind};
use angstrom_types::orders::{OrderLocation, OrderStatus};

use angstrom_types::sol_bindings::grouped_orders::AllOrders;
use futures::Stream;

pub trait OrderApi {
    /// Submit any type of order
    fn send_order(
        &self,
        order: AllOrders,
    ) -> impl std::future::Future<Output = eyre::Result<bool>> + Send;

    fn pending_order(
        &self,
        from: Address,
    ) -> impl std::future::Future<Output = eyre::Result<Vec<AllOrders>>> + Send;

    fn cancel_order(
        &self,
        request: CancelOrderRequest,
    ) -> impl std::future::Future<Output = eyre::Result<bool>> + Send;

    fn estimate_gas(
        &self,
        order: AllOrders,
    ) -> impl std::future::Future<Output = eyre::Result<GasEstimateResponse>> + Send;

    async fn order_status(&self, order_hash: B256) -> eyre::Result<Option<OrderStatus>>;

    async fn orders_by_pair(
        &self,
        pair: FixedBytes<32>,
        location: OrderLocation,
    ) -> eyre::Result<Vec<AllOrders>>;

    async fn subscribe_orders(
        &self,
        kind: HashSet<OrderSubscriptionKind>,
        filters: HashSet<OrderSubscriptionFilter>,
    ) -> eyre::Result<impl Stream<Item = AllOrders> + Send>;

    // MULTI CALL
    async fn send_orders(&self, orders: Vec<AllOrders>) -> eyre::Result<Vec<bool>>;

    async fn pending_orders(&self, from: Vec<Address>) -> eyre::Result<Vec<AllOrders>>;

    async fn cancel_orders(&self, request: Vec<CancelOrderRequest>) -> eyre::Result<Vec<bool>>;

    async fn estimate_gas_of_orders(
        &self,
        orders: Vec<AllOrders>,
    ) -> eyre::Result<Vec<GasEstimateResponse>>;

    async fn status_of_orders(
        &self,
        order_hashes: Vec<B256>,
    ) -> eyre::Result<Vec<Option<OrderStatus>>>;

    async fn orders_by_pairs(
        &self,
        pair_with_location: Vec<(FixedBytes<32>, OrderLocation)>,
    ) -> eyre::Result<Vec<AllOrders>>;
}
