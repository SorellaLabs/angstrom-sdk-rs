use std::collections::HashSet;

use alloy_primitives::{Address, B256};
use angstrom_rpc::{
    api::{GasEstimateResponse, OrderApiClient},
    types::{OrderSubscriptionFilter, OrderSubscriptionKind, OrderSubscriptionResult}
};
use angstrom_types::{
    orders::{CancelOrderRequest, OrderLocation, OrderStatus},
    primitive::PoolId,
    sol_bindings::grouped_orders::AllOrders
};
use futures::{Stream, StreamExt, TryStreamExt};
use jsonrpsee_http_client::HttpClient;
use validation::order::OrderPoolNewOrderResult;

pub trait AngstromNodeApi {
    fn rpc_provider(&self) -> HttpClient;

    async fn send_order(&self, order: AllOrders) -> eyre::Result<OrderPoolNewOrderResult> {
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

    async fn orders_by_pool_id(
        &self,
        pool_id: PoolId,
        location: OrderLocation
    ) -> eyre::Result<Vec<AllOrders>> {
        let provider = self.rpc_provider();
        Ok(provider.orders_by_pool_id(pool_id, location).await?)
    }

    async fn subscribe_orders(
        &self,
        kind: HashSet<OrderSubscriptionKind>,
        filters: HashSet<OrderSubscriptionFilter>
    ) -> eyre::Result<impl Stream<Item = eyre::Result<OrderSubscriptionResult>>> {
        let provider = self.rpc_provider();

        Ok(provider
            .subscribe_orders(kind, filters)
            .await?
            .map(|order| Ok(order?))
            .into_stream())
    }

    async fn send_orders(
        &self,
        orders: Vec<AllOrders>
    ) -> eyre::Result<Vec<OrderPoolNewOrderResult>> {
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
        orders: Vec<AllOrders>
    ) -> eyre::Result<Vec<GasEstimateResponse>> {
        let provider = self.rpc_provider();
        Ok(provider.estimate_gas_of_orders(orders).await?)
    }

    async fn status_of_orders(
        &self,
        order_hashes: Vec<B256>
    ) -> eyre::Result<Vec<Option<OrderStatus>>> {
        let provider = self.rpc_provider();
        Ok(provider.status_of_orders(order_hashes).await?)
    }

    async fn orders_by_pool_ids(
        &self,
        pool_ids_with_location: Vec<(PoolId, OrderLocation)>
    ) -> eyre::Result<Vec<AllOrders>> {
        let provider = self.rpc_provider();
        Ok(provider.orders_by_pool_ids(pool_ids_with_location).await?)
    }
}

#[cfg(test)]
mod tests {

    use std::task::Poll;

    use alloy_primitives::U256;
    use alloy_provider::Provider;
    use angstrom_types::sol_bindings::{
        grouped_orders::{FlashVariants, GroupedVanillaOrder},
        rpc_orders::TopOfBlockOrder,
        RawPoolOrder
    };
    use testing_tools::order_generator::GeneratedPoolOrders;

    use super::*;
    use crate::{
        apis::data_api::AngstromDataApi,
        providers::{AngstromProvider, EthRpcProvider},
        test_utils::{make_generator, spawn_angstrom_provider, spawn_ws_provider}
    };

    fn get_flash_order(orders: &[GeneratedPoolOrders]) -> FlashVariants {
        orders
            .iter()
            .flat_map(|book| book.book.clone())
            .filter_map(|order| match order {
                GroupedVanillaOrder::KillOrFill(or) => Some(or.clone()),
                _ => None
            })
            .next()
            .unwrap()
    }

    fn get_tob_order(orders: &[GeneratedPoolOrders]) -> TopOfBlockOrder {
        orders.first().unwrap().tob.clone()
    }

    struct AllOrdersSent {
        tob:  AllOrders,
        user: AllOrders
    }

    impl AllOrdersSent {
        async fn send_orders<P>(
            eth_provider: &EthRpcProvider<P>,
            angstrom_provider: &AngstromProvider
        ) -> eyre::Result<Self>
        where
            P: Provider + Clone
        {
            let (generator, _rx) = make_generator(eth_provider).await.unwrap();
            let orders = generator.generate_orders();

            let tob_order = AllOrders::TOB(get_tob_order(&orders));
            let tob_order_sent = angstrom_provider
                .send_order(tob_order.clone())
                .await
                .unwrap();
            assert!(tob_order_sent.is_valid());

            let user_order = AllOrders::Flash(get_flash_order(&orders));
            let user_order_sent = angstrom_provider
                .send_order(user_order.clone())
                .await
                .unwrap();
            assert!(user_order_sent.is_valid());

            Ok(Self { tob: tob_order, user: user_order })
        }
    }

    #[tokio::test]
    async fn test_send_order() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let _ = AllOrdersSent::send_orders(&eth_provider, &angstrom_provider)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_pending_order() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let orders = AllOrdersSent::send_orders(&eth_provider, &angstrom_provider)
            .await
            .unwrap();

        let pending_tob_order = angstrom_provider
            .pending_order(orders.tob.from())
            .await
            .unwrap();
        assert_eq!(vec![orders.tob.clone()], pending_tob_order);

        let pending_user_orders = angstrom_provider
            .pending_order(orders.user.from())
            .await
            .unwrap();
        assert_eq!(vec![orders.user.clone()], pending_user_orders);
    }

    #[tokio::test]
    async fn test_cancel_order() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let orders = AllOrdersSent::send_orders(&eth_provider, &angstrom_provider)
            .await
            .unwrap();

        let canceled_tob_order = angstrom_provider
            .cancel_order(CancelOrderRequest {
                signature:    orders.tob.order_signature().unwrap(),
                user_address: orders.tob.from(),
                order_id:     orders.tob.order_hash()
            })
            .await
            .unwrap();
        assert!(canceled_tob_order);

        let canceled_user_orders = angstrom_provider
            .cancel_order(CancelOrderRequest {
                signature:    orders.user.order_signature().unwrap(),
                user_address: orders.user.from(),
                order_id:     orders.user.order_hash()
            })
            .await
            .unwrap();
        assert!(canceled_user_orders);
    }

    #[tokio::test]
    async fn test_estimate_gas() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let (generator, _rx) = make_generator(&eth_provider).await.unwrap();
        let orders = generator.generate_orders();

        let tob_order = AllOrders::TOB(get_tob_order(&orders));
        let tob_order_gas_estimation = angstrom_provider.estimate_gas(tob_order).await.unwrap();
        assert_eq!(tob_order_gas_estimation, GasEstimateResponse { gas_units: 0, gas: U256::ZERO });

        let user_order = AllOrders::Flash(get_flash_order(&orders));
        let user_order_gas_estimation = angstrom_provider.estimate_gas(user_order).await.unwrap();
        assert_eq!(
            user_order_gas_estimation,
            GasEstimateResponse { gas_units: 0, gas: U256::ZERO }
        );
    }

    #[tokio::test]
    async fn test_order_status() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let orders = AllOrdersSent::send_orders(&eth_provider, &angstrom_provider)
            .await
            .unwrap();

        let status_tob_order = angstrom_provider
            .order_status(orders.tob.order_hash())
            .await
            .unwrap();
        assert_eq!(status_tob_order, Some(OrderStatus::Pending));

        let status_user_order = angstrom_provider
            .order_status(orders.user.order_hash())
            .await
            .unwrap();
        assert_eq!(status_user_order, Some(OrderStatus::Pending));
    }

    #[tokio::test]
    async fn test_order_by_pool_id() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let orders = AllOrdersSent::send_orders(&eth_provider, &angstrom_provider)
            .await
            .unwrap();

        let tob_pool_id = eth_provider
            .pool_id(orders.tob.token_in(), orders.tob.token_out())
            .await
            .unwrap();
        let tob_orders = angstrom_provider
            .orders_by_pool_id(tob_pool_id, orders.tob.order_location())
            .await
            .unwrap();
        assert_eq!(vec![orders.tob.clone()], tob_orders);

        let user_pool_id = eth_provider
            .pool_id(orders.user.token_in(), orders.user.token_out())
            .await
            .unwrap();
        let user_orders = angstrom_provider
            .orders_by_pool_id(user_pool_id, orders.user.order_location())
            .await
            .unwrap();
        assert_eq!(vec![orders.user.clone()], user_orders);
    }

    #[tokio::test]
    async fn test_subscribe_orders() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let mut sub_stream = angstrom_provider
            .subscribe_orders(HashSet::new(), HashSet::new())
            .await
            .unwrap();

        let order_cycles = 2;

        let stream_fut = std::future::poll_fn(|cx| {
            let mut i = order_cycles * 2;

            loop {
                if let Poll::Ready(Some(val)) = sub_stream.poll_next_unpin(cx) {
                    let _ = val.unwrap();
                    i -= 1;
                    if i == 0 {
                        return Poll::Ready(());
                    }
                }
            }
        });

        let order_send_fut = async {
            for _ in 0..order_cycles {
                let _ = AllOrdersSent::send_orders(&eth_provider, &angstrom_provider)
                    .await
                    .unwrap();
            }
        };

        tokio::join!(order_send_fut, stream_fut);
    }
}
