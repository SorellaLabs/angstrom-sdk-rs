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

#[cfg(all(test, feature = "example-utils"))]
mod tests {

    use std::task::Poll;

    use alloy_provider::Provider;
    use angstrom_types_primitives::sol_bindings::{RawPoolOrder, rpc_orders::TopOfBlockOrder};
    use testing_tools::order_generator::GeneratedPoolOrders;

    use super::*;
    use crate::{
        apis::data_api::AngstromL1DataApi,
        providers::backend::AngstromProvider,
        test_utils::{
            AngstromOrderApiClientClone, filler_orders::make_order_generator, spawn_angstrom_api
        },
        types::errors::AngstromSdkError
    };

    fn get_flash_order(orders: &[GeneratedPoolOrders]) -> AllOrders {
        orders
            .iter()
            .flat_map(|book| book.book.clone())
            .find(|order| order.deadline().is_none())
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
        async fn send_orders<P, T>(
            provider: &AngstromProvider<P, T>
        ) -> Result<Self, AngstromSdkError>
        where
            P: Provider + Clone,
            T: AngstromOrderApiClientClone
        {
            let (generator, _rx) = make_order_generator(provider).await.unwrap();
            let orders = generator.generate_orders().await;

            let tob_order = AllOrders::TOB(get_tob_order(&orders));
            let tob_order_sent = provider.send_order(tob_order.clone()).await;
            assert!(
                tob_order_sent.is_ok()
                    || matches!(tob_order_sent, Err(AngstromSdkError::AngstromRpc(_))),
                "{tob_order_sent:?}"
            );

            let user_order = get_flash_order(&orders);
            let user_order_sent = provider.send_order(user_order.clone()).await;
            assert!(
                user_order_sent.is_ok()
                    || matches!(user_order_sent, Err(AngstromSdkError::AngstromRpc(_))),
                "{user_order_sent:?}"
            );

            Ok(Self { tob: tob_order, user: user_order })
        }
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_send_order() {
        let provider = spawn_angstrom_api().await.unwrap();

        let _ = AllOrdersSent::send_orders(provider.angstrom_provider())
            .await
            .unwrap();
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_pending_order() {
        let provider = spawn_angstrom_api().await.unwrap();

        let orders = AllOrdersSent::send_orders(provider.angstrom_provider())
            .await
            .unwrap();

        let pending_tob_order = provider.pending_order(orders.tob.from()).await.unwrap();
        assert_eq!(
            vec![PendingOrder { order_id: orders.tob.order_hash(), order: orders.tob.clone() }],
            pending_tob_order
        );

        let pending_user_orders = provider.pending_order(orders.user.from()).await.unwrap();
        assert_eq!(
            vec![PendingOrder {
                order_id: orders.user.order_hash(),
                order:    orders.user.clone()
            }],
            pending_user_orders
        );
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_cancel_order() {
        let provider = spawn_angstrom_api().await.unwrap();

        let orders = AllOrdersSent::send_orders(provider.angstrom_provider())
            .await
            .unwrap();

        let canceled_tob_order = provider
            .cancel_order(CancelOrderRequest {
                signature:    orders.tob.order_signature().unwrap().as_bytes().into(),
                user_address: orders.tob.from(),
                order_id:     orders.tob.order_hash()
            })
            .await
            .unwrap();
        assert!(canceled_tob_order);

        let canceled_user_orders = provider
            .cancel_order(CancelOrderRequest {
                signature:    orders.user.order_signature().unwrap().as_bytes().into(),
                user_address: orders.user.from(),
                order_id:     orders.user.order_hash()
            })
            .await
            .unwrap();
        assert!(canceled_user_orders);
    }

    // #[tokio::test]
    // async fn test_estimate_gas() {
    //     let provider = spawn_angstrom_api().await.unwrap();

    //     let (generator, _rx) = make_order_generator(provider.angstrom_provider())
    //         .await
    //         .unwrap();
    //     let orders = generator.generate_orders().await;

    //     let tob_order = AllOrders::TOB(get_tob_order(&orders));
    //     let tokens = sort_tokens(tob_order.token_in(), tob_order.token_out());
    //     let tob_order_gas_estimation = provider
    //         .estimate_gas(tob_order.is_tob(), false, tokens.0, tokens.1)
    //         .await
    //         .unwrap();
    //     assert_eq!(tob_order_gas_estimation, U256::ZERO);

    //     let user_order = get_flash_order(&orders);
    //     let tokens = sort_tokens(user_order.token_in(), user_order.token_out());
    //     let user_order_gas_estimation = provider
    //         .estimate_gas(!user_order.is_tob(), false, tokens.0, tokens.1)
    //         .await
    //         .unwrap();
    //     assert_eq!(user_order_gas_estimation, U256::ZERO);
    // }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_order_status() {
        let provider = spawn_angstrom_api().await.unwrap();

        let orders = AllOrdersSent::send_orders(provider.angstrom_provider())
            .await
            .unwrap();

        let status_tob_order = provider
            .order_status(orders.tob.order_hash())
            .await
            .unwrap();
        assert_eq!(status_tob_order, OrderStatus::Pending);

        let status_user_order = provider
            .order_status(orders.user.order_hash())
            .await
            .unwrap();
        assert_eq!(status_user_order, OrderStatus::Pending);
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_order_by_pool_id() {
        let provider = spawn_angstrom_api().await.unwrap();

        let orders = AllOrdersSent::send_orders(provider.angstrom_provider())
            .await
            .unwrap();

        let tob_pool_id = provider
            .pool_id(orders.tob.token_in(), orders.tob.token_out(), None)
            .await
            .unwrap();
        let tob_orders = provider
            .orders_by_pool_id(tob_pool_id, orders.tob.order_location())
            .await
            .unwrap();
        assert_eq!(vec![orders.tob.clone()], tob_orders);

        let user_pool_id = provider
            .pool_id(orders.user.token_in(), orders.user.token_out(), None)
            .await
            .unwrap();
        let user_orders = provider
            .orders_by_pool_id(user_pool_id, orders.user.order_location())
            .await
            .unwrap();
        assert_eq!(vec![orders.user.clone()], user_orders);
    }

    #[serial_test::serial]
    #[tokio::test]
    async fn test_subscribe_orders() {
        let provider = spawn_angstrom_api().await.unwrap();

        let mut sub_stream = provider
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
                let _ = AllOrdersSent::send_orders(provider.angstrom_provider())
                    .await
                    .unwrap();
            }
        };

        tokio::join!(order_send_fut, stream_fut);
    }
}
