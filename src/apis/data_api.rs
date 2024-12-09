use crate::apis::utils::pool_config_store;
use crate::providers::EthProvider;
use alloy_primitives::Address;
use alloy_primitives::FixedBytes;

use alloy_rpc_types::BlockTransactionsKind;
use angstrom_types::contract_bindings::controller_v_1::ControllerV1;

use futures::StreamExt;

use crate::types::*;

pub trait AngstromDataApi: EthProvider {
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        let partial_keys = pool_config_store(self)
            .await?
            .all_entries()
            .iter()
            .map(|val| FixedBytes::from(*val.pool_partial_key))
            .collect::<Vec<_>>();

        let all_pools_call = self
            .view_call(
                CONTROLLER_V1_ADDRESS,
                ControllerV1::getAllPoolsCall {
                    storeKeys: partial_keys,
                },
            )
            .await?;

        Ok(all_pools_call
            ._0
            .into_iter()
            .map(|val| TokenPairInfo {
                token0: val.asset0,
                token1: val.asset1,
                is_active: true,
            })
            .collect())
    }

    async fn pool_metadata(&self, token0: Address, token1: Address) -> eyre::Result<PoolMetadata> {
        let config_store = pool_config_store(self).await?;
        let pool_config_store = config_store.get_entry(token0, token1).ok_or(eyre::eyre!(
            "no config store entry for tokens {token0:?} - {token1:?}"
        ))?;

        Ok(PoolMetadata::new(token0, token1, pool_config_store))
    }

    async fn historical_orders(
        &self,
        filter: &HistoricalOrdersFilter,
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        let pool_stores = &AngstromPoolTokenIndexToPair::new_with_tokens(self, filter).await?;

        let start_block = filter.from_block.unwrap_or(ANGSTROM_DEPLOYED_BLOCK);
        let end_block = if let Some(e) = filter.to_block {
            e
        } else {
            self.current_block_number().await?
        };

        let mut block_stream = futures::stream::iter(start_block..end_block)
            .map(|bn| async move {
                let block = self.get_block(bn, BlockTransactionsKind::Full).await?;

                Ok::<_, eyre::ErrReport>(filter.filter_block(block, pool_stores))
            })
            .buffer_unordered(10);

        let mut all_orders = Vec::new();
        while let Some(val) = block_stream.next().await {
            all_orders.extend(val?);
        }

        Ok(all_orders)
    }
}
