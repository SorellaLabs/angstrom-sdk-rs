use std::future::Future;

use alloy_primitives::{Address, TxHash, B256};
use angstrom_types::sol_bindings::grouped_orders::AllOrders;

use crate::types::*;

pub trait InfoApi {
    fn all_token_pairs(&self) -> impl Future<Output = eyre::Result<Vec<TokenPairInfo>>> + Send;

    fn active_token_pairs(&self) -> impl Future<Output = eyre::Result<Vec<TokenPairInfo>>> + Send;

    fn pool_metadata(
        &self,
        token0: Address,
        token1: Address,
    ) -> impl Future<Output = eyre::Result<PoolMetadata>> + Send;

    fn historical_trades(
        &self,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> impl Future<Output = eyre::Result<Vec<AllOrders>>> + Send;

    fn historical_trade(
        &self,
        order_hash: B256,
        tx_hash: TxHash,
    ) -> impl Future<Output = eyre::Result<AllOrders>> + Send;
}
