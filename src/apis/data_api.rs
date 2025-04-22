use std::collections::{HashMap, HashSet};

use crate::types::*;
use alloy_eips::BlockId;
use alloy_primitives::aliases::{I24, U24};
use alloy_primitives::{Address, FixedBytes};
use alloy_provider::Provider;
use angstrom_types::contract_bindings::{
    controller_v_1::ControllerV1, mintable_mock_erc_20::MintableMockERC20,
};
use angstrom_types::contract_payloads::angstrom::AngstromPoolConfigStore;
use angstrom_types::{contract_bindings::angstrom::Angstrom::PoolKey, primitive::PoolId};
use futures::{StreamExt, TryFutureExt};
use std::sync::Arc;
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use super::utils::*;

// #[auto_impl::auto_impl(&)]
pub trait AngstromDataApi {
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>>;

    async fn all_tokens(&self) -> eyre::Result<Vec<TokenInfoWithMeta>>;

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey>;

    async fn all_pool_keys(&self) -> eyre::Result<Vec<PoolKey>> {
        let (config_store, all_token_pairs) =
            tokio::try_join!(self.pool_config_store(None), self.all_token_pairs())?;

        let tokens_to_partial_keys = all_token_pairs
            .into_iter()
            .map(|tokens| {
                (
                    AngstromPoolConfigStore::derive_store_key(tokens.token0, tokens.token1),
                    (tokens.token0, tokens.token1),
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(config_store
            .all_entries()
            .into_iter()
            .map(|entry| {
                let (k, v) = entry.pair();
                let (token0, token1) = tokens_to_partial_keys.get(k).unwrap();

                PoolKey {
                    currency0: *token0,
                    currency1: *token1,
                    fee: U24::from(v.fee_in_e6),
                    tickSpacing: I24::unchecked_from(v.tick_spacing),
                    hooks: ANGSTROM_ADDRESS,
                }
            })
            .collect())
    }

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
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)>;

    async fn all_pool_data(
        &self,
        block_number: Option<u64>,
    ) -> eyre::Result<Vec<(u64, EnhancedUniswapPool<DataLoader>)>> {
        let token_pairs = self.all_token_pairs().await?;

        let pools = futures::future::try_join_all(
            token_pairs
                .into_iter()
                .map(|pair| self.pool_data(pair.token0, pair.token1, block_number)),
        )
        .await?;

        Ok(pools)
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>,
    ) -> eyre::Result<AngstromPoolConfigStore>;
}

impl<P: Provider> AngstromDataApi for P {
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        let config_store = self.pool_config_store(None).await?;
        let partial_key_entries = config_store.all_entries();

        let all_pools_call = futures::future::try_join_all(partial_key_entries.iter().map(|key| {
            view_call(
                self,
                CONTROLLER_V1_ADDRESS,
                ControllerV1::poolsCall { key: FixedBytes::from(*key.pool_partial_key) },
            )
        }))
        .await?;

        Ok(all_pools_call
            .into_iter()
            .map(|val_res| {
                val_res.map(|val| TokenPairInfo {
                    token0: val.asset0,
                    token1: val.asset1,
                    is_active: true,
                })
            })
            .collect::<Result<Vec<_>, _>>()?)
    }

    async fn all_tokens(&self) -> eyre::Result<Vec<TokenInfoWithMeta>> {
        let all_tokens_addresses = self
            .all_token_pairs()
            .await?
            .into_iter()
            .flat_map(|val| [val.token0, val.token1])
            .collect::<HashSet<_>>();

        Ok(futures::future::try_join_all(all_tokens_addresses.into_iter().map(|address| {
            view_call(self, address, MintableMockERC20::symbolCall {}).and_then(
                async move |val_res| {
                    Ok(val_res.map(|val| TokenInfoWithMeta { address, symbol: val }))
                },
            )
        }))
        .await?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?)
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        let (token0, token1) = sort_tokens(token0, token1);

        let config_store = self.pool_config_store(None).await?;
        let pool_config_store = config_store
            .get_entry(token0, token1)
            .ok_or(eyre::eyre!("no config store entry for tokens {token0:?} - {token1:?}"))?;

        Ok(PoolKey {
            currency0: token0,
            currency1: token1,
            fee: U24::from(pool_config_store.fee_in_e6),
            tickSpacing: I24::unchecked_from(pool_config_store.tick_spacing),
            hooks: ANGSTROM_ADDRESS,
        })
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        let filter = &filter;
        let pool_stores = &AngstromPoolTokenIndexToPair::new_with_tokens(self, filter).await?;

        let start_block = filter.from_block.unwrap_or(ANGSTROM_DEPLOYED_BLOCK);
        let end_block =
            if let Some(e) = filter.to_block { e } else { self.get_block_number().await? };

        let mut block_stream = futures::stream::iter(start_block..end_block)
            .map(|bn| async move {
                let block = self
                    .get_block(bn.into())
                    .full()
                    .await?
                    .ok_or(eyre::eyre!("block number {bn} not found"))?;

                Ok::<_, eyre::ErrReport>(filter.filter_block(block, pool_stores))
            })
            .buffer_unordered(10);

        let mut all_orders = Vec::new();
        while let Some(val) = block_stream.next().await {
            all_orders.extend(val?);
        }

        Ok(all_orders)
    }

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        let (token0, token1) = sort_tokens(token0, token1);

        let mut pool_key = self.pool_key(token0, token1).await?;
        pool_key.fee = U24::from(0x800000);
        let pool_id: PoolId = pool_key.clone().into();

        let data_loader =
            DataLoader::new_with_registry(pool_id, vec![pool_key].into(), POOL_MANAGER_ADDRESS);

        let mut enhanced_uni_pool = EnhancedUniswapPool::new(data_loader, 200);

        let block_number =
            if let Some(bn) = block_number { bn } else { self.get_block_number().await? };

        enhanced_uni_pool
            .initialize(Some(block_number), Arc::new(self))
            .await?;

        Ok((block_number, enhanced_uni_pool))
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>,
    ) -> eyre::Result<AngstromPoolConfigStore> {
        AngstromPoolConfigStore::load_from_chain(
            ANGSTROM_ADDRESS,
            block_number.map(Into::into).unwrap_or(BlockId::latest()),
            self,
        )
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))
    }
}
