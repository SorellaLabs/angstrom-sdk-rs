use alloy_primitives::{Address, FixedBytes, B256};
use angstrom_rpc::api::{GasEstimateResponse, OrderApiClient};
use angstrom_rpc::types::{
    OrderSubscriptionFilter, OrderSubscriptionKind, OrderSubscriptionResult,
};
use angstrom_types::orders::{CancelOrderRequest, OrderLocation, OrderStatus};
use angstrom_types::sol_bindings::grouped_orders::AllOrders;
use futures::{Stream, StreamExt, TryStreamExt};
use jsonrpsee_http_client::HttpClient;
use std::collections::HashSet;
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

    async fn send_orders(
        &self,
        orders: Vec<AllOrders>,
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
    #![allow(unused)]

    use alloy_primitives::U256;
    use angstrom_types::sol_bindings::RawPoolOrder;
    use angstrom_types::sol_bindings::{
        grouped_orders::{FlashVariants, GroupedVanillaOrder, StandingVariants},
        rpc_orders::TopOfBlockOrder,
    };

    use testing_tools::order_generator::GeneratedPoolOrders;

    use crate::test_utils::{make_generator, spawn_angstrom_provider, spawn_ws_provider};

    use super::*;

    fn get_standing_order(orders: &[GeneratedPoolOrders]) -> StandingVariants {
        orders
            .iter()
            .flat_map(|book| book.book.clone())
            .filter_map(|order| match order {
                GroupedVanillaOrder::Standing(or) => Some(or.clone()),
                _ => None,
            })
            .next()
            .unwrap()
    }

    fn get_flash_order(orders: &[GeneratedPoolOrders]) -> FlashVariants {
        orders
            .iter()
            .flat_map(|book| book.book.clone())
            .filter_map(|order| match order {
                GroupedVanillaOrder::KillOrFill(or) => Some(or.clone()),
                _ => None,
            })
            .next()
            .unwrap()
    }

    fn get_tob_order(orders: &[GeneratedPoolOrders]) -> TopOfBlockOrder {
        orders.first().clone().unwrap().tob.clone()
    }

    #[tokio::test]
    async fn test_send_order() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let generator = make_generator(&eth_provider).await.unwrap();
        let orders = generator.generate_orders();

        let tob_order = AllOrders::TOB(get_tob_order(&orders));
        let tob_order_sent = angstrom_provider.send_order(tob_order).await.unwrap();
        assert!(tob_order_sent.is_valid());

        let user_order = AllOrders::Flash(get_flash_order(&orders));
        let user_order_sent = angstrom_provider.send_order(user_order).await.unwrap();
        assert!(user_order_sent.is_valid());
    }

    #[tokio::test]
    async fn test_pending_order() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let generator = make_generator(&eth_provider).await.unwrap();
        let orders = generator.generate_orders();

        let tob_order = AllOrders::TOB(get_tob_order(&orders));
        let tob_order_sent = angstrom_provider
            .send_order(tob_order.clone())
            .await
            .unwrap();
        assert!(tob_order_sent.is_valid());
        let pending_tob_order = angstrom_provider
            .pending_order(tob_order.from())
            .await
            .unwrap();
        assert_eq!(vec![tob_order.clone()], pending_tob_order);

        let user_order = AllOrders::Flash(get_flash_order(&orders));
        let user_order_sent = angstrom_provider
            .send_order(user_order.clone())
            .await
            .unwrap();
        assert!(user_order_sent.is_valid());
        let pending_user_orders = angstrom_provider
            .pending_order(user_order.from())
            .await
            .unwrap();
        assert_eq!(vec![user_order.clone()], pending_user_orders);
    }

    #[tokio::test]
    async fn test_cancel_order() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let generator = make_generator(&eth_provider).await.unwrap();
        let orders = generator.generate_orders();

        let tob_order = AllOrders::TOB(get_tob_order(&orders));
        let tob_order_sent = angstrom_provider
            .send_order(tob_order.clone())
            .await
            .unwrap();
        assert!(tob_order_sent.is_valid());
        let canceled_tob_order = angstrom_provider
            .cancel_order(CancelOrderRequest {
                signature: tob_order.order_signature().unwrap(),
                user_address: tob_order.from(),
                order_id: tob_order.order_hash(),
            })
            .await
            .unwrap();
        assert!(canceled_tob_order);

        let user_order = AllOrders::Flash(get_flash_order(&orders));
        let user_order_sent = angstrom_provider
            .send_order(user_order.clone())
            .await
            .unwrap();
        assert!(user_order_sent.is_valid());
        let canceled_user_orders = angstrom_provider
            .cancel_order(CancelOrderRequest {
                signature: user_order.order_signature().unwrap(),
                user_address: user_order.from(),
                order_id: user_order.order_hash(),
            })
            .await
            .unwrap();
        assert!(canceled_user_orders);
    }

    #[tokio::test]
    async fn test_estimate_gas() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let generator = make_generator(&eth_provider).await.unwrap();
        let orders = generator.generate_orders();

        let tob_order = AllOrders::TOB(get_tob_order(&orders));
        let tob_order_gas_estimation = angstrom_provider.estimate_gas(tob_order).await.unwrap();
        assert_eq!(
            tob_order_gas_estimation,
            GasEstimateResponse {
                gas_units: 0,
                gas: U256::ZERO
            }
        );

        let user_order = AllOrders::Flash(get_flash_order(&orders));
        let user_order_gas_estimation = angstrom_provider.estimate_gas(user_order).await.unwrap();
        assert_eq!(
            user_order_gas_estimation,
            GasEstimateResponse {
                gas_units: 0,
                gas: U256::ZERO
            }
        );
    }

    #[tokio::test]
    async fn test_order_status() {
        let eth_provider = spawn_ws_provider().await.unwrap();
        let angstrom_provider = spawn_angstrom_provider().await.unwrap();

        let generator = make_generator(&eth_provider).await.unwrap();
        let orders = generator.generate_orders();

        let tob_order = AllOrders::TOB(get_tob_order(&orders));
        let tob_order_sent = angstrom_provider
            .send_order(tob_order.clone())
            .await
            .unwrap();
        assert!(tob_order_sent.is_valid());
        let status_tob_order = angstrom_provider
            .order_status(tob_order.order_hash())
            .await
            .unwrap();
        assert_eq!(status_tob_order, Some(OrderStatus::Pending));

        let user_order = AllOrders::Flash(get_flash_order(&orders));
        let user_order_sent = angstrom_provider
            .send_order(user_order.clone())
            .await
            .unwrap();
        assert!(user_order_sent.is_valid());
        let status_user_order = angstrom_provider
            .order_status(user_order.order_hash())
            .await
            .unwrap();
        assert_eq!(status_user_order, Some(OrderStatus::Pending));
    }
}
