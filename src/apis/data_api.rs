use alloy_primitives::Address;
use angstrom_types::{contract_bindings::angstrom::Angstrom::PoolKey, primitive::PoolId};
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::types::*;

pub trait AngstromDataApi {
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>>;

    async fn all_tokens(&self) -> eyre::Result<Vec<TokenInfoWithMeta>>;

    // async fn binance_price(&self, token_address: Address) -> eyre::Result<BinanceTokenPrice>;

    // async fn binance_prices(
    //     &self,
    //     token_addresses: Vec<Address>,
    // ) -> eyre::Result<Vec<BinanceTokenPrice>> {
    //     Ok(futures::future::try_join_all(
    //         token_addresses
    //             .into_iter()
    //             .map(|addr| self.binance_price(addr)),
    //     )
    //     .await?)
    // }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey>;

    async fn pool_id(&self, token0: Address, token1: Address) -> eyre::Result<PoolId> {
        self.pool_key(token0, token1).await.map(Into::into)
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
    ) -> eyre::Result<Vec<HistoricalOrders>>;

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<EnhancedUniswapPool<DataLoader>>;

    async fn all_pool_data(
        &self,
        block_number: Option<u64>,
    ) -> eyre::Result<Vec<EnhancedUniswapPool<DataLoader>>> {
        let token_pairs = self.all_token_pairs().await?;

        let pools = futures::future::try_join_all(
            token_pairs
                .into_iter()
                .map(|pair| self.pool_data(pair.token0, pair.token1, block_number)),
        )
        .await?;

        Ok(pools)
    }
}
