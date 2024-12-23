use alloy_primitives::Address;
use angstrom_types::contract_bindings::angstrom::Angstrom::PoolKey;
use angstrom_types::primitive::PoolId;

use uniswap_v4::uniswap::pool::EnhancedUniswapPool;
use uniswap_v4::uniswap::pool_data_loader::DataLoader;

use crate::types::*;

pub trait AngstromDataApi {
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>>;

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey>;

    async fn pool_id(&self, token0: Address, token1: Address) -> eyre::Result<PoolId> {
        self.pool_key(token0, token1).await.map(Into::into)
    }

    async fn historical_orders(
        &self,
        filter: &HistoricalOrdersFilter,
    ) -> eyre::Result<Vec<HistoricalOrders>>;

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<EnhancedUniswapPool<DataLoader<PoolId>, PoolId>>;
}
