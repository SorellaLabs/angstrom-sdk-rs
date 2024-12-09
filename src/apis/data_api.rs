use std::future::Future;

use crate::providers::EthProvider;
use alloy_primitives::{Address, TxHash, B256};
use alloy_primitives::{FixedBytes, U256};

use alloy_rpc_types::BlockTransactionsKind;
use angstrom_types::contract_bindings::controller_v_1::ControllerV1;

use angstrom_types::contract_payloads::angstrom::AngstromPoolConfigStore;
use angstrom_types::sol_bindings::grouped_orders::AllOrders;
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
        let start_block = filter.from_block.unwrap_or(ANGSTROM_DEPLOYED_BLOCK);
        let end_block = if let Some(e) = filter.to_block {
            e
        } else {
            self.current_block_number().await?
        };

        let mut block_stream = futures::stream::iter(start_block..end_block)
            .map(|bn| async move {
                let block = self.get_block(bn, BlockTransactionsKind::Full).await?;

                Ok::<_, eyre::ErrReport>(filter.filter_block(block))
            })
            .buffer_unordered(10);

        let mut all_orders = Vec::new();
        while let Some(val) = block_stream.next().await {
            all_orders.extend(val?);
        }

        Ok(all_orders)
    }
}

async fn pool_config_store<E: EthProvider>(provider: &E) -> eyre::Result<AngstromPoolConfigStore> {
    let value = provider
        .get_storage_at(ANGSTROM_ADDRESS, U256::from(POOL_CONFIG_STORE_SLOT))
        .await?;

    let value_bytes: [u8; 32] = value.to_be_bytes();
    let config_store_address = Address::from(<[u8; 20]>::try_from(&value_bytes[4..24])?);

    let code = provider.get_code_at(config_store_address).await?;

    AngstromPoolConfigStore::try_from(code.0.to_vec().as_slice()).map_err(|e| eyre::eyre!("{e:?}"))
}
