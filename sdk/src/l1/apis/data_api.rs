use std::collections::HashMap;

use alloy_network::Ethereum;
use alloy_primitives::{
    Address, TxHash,
    aliases::{I24, U24}
};
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager::{self, PoolKey},
    contract_payloads::angstrom::{
        AngstromBundle, AngstromPoolConfigStore, AngstromPoolPartialKey
    },
    primitive::PoolId
};
use itertools::Itertools;
use uni_v4::FeeConfiguration;
use uniswap_storage::v4::UnpackedSlot0;

use crate::{
    l1::{AngstromL1Chain, types::*},
    types::{common::*, pool_tick_loaders::PoolTickDataLoader}
};

#[async_trait::async_trait]
pub trait AngstromL1DataApi: PoolTickDataLoader<Ethereum> + Send + Sized {
    async fn all_token_pairs(
        &self,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<TokenPair>> {
        let config_store = self.pool_config_store(block_number, chain).await?;
        self.all_token_pairs_with_config_store(config_store, block_number, chain)
            .await
    }

    async fn all_token_pairs_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<TokenPair>>;

    async fn all_tokens(
        &self,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<Address>> {
        let config_store = self.pool_config_store(block_number, chain).await?;
        self.all_tokens_with_config_store(config_store, block_number, chain)
            .await
    }

    async fn all_tokens_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<Address>> {
        Ok(self
            .all_token_pairs_with_config_store(config_store, block_number, chain)
            .await?
            .into_iter()
            .flat_map(|val| [val.token0, val.token1])
            .unique()
            .collect())
    }

    async fn pool_key_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<PoolKeyWithAngstromFee>;

    async fn pool_key_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<PoolKeyWithAngstromFee> {
        let config_store = self.pool_config_store(block_number, chain).await?;
        self.pool_key_by_pool_id_with_config_store(pool_id, config_store, block_number, chain)
            .await
    }

    async fn pool_key_by_pool_id_with_config_store(
        &self,
        pool_id: PoolId,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<PoolKeyWithAngstromFee> {
        self.all_pool_keys_with_config_store(config_store, block_number, chain)
            .await?
            .into_iter()
            .find(|pool_key| pool_id == PoolId::from(pool_key))
            .ok_or_else(|| eyre::eyre!("no pool key for pool_id: {pool_id:?}"))
    }

    async fn tokens_by_partial_pool_key(
        &self,
        partial_pool_key: AngstromPoolPartialKey,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<TokenPair>;

    async fn all_pool_keys(
        &self,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<PoolKeyWithAngstromFee>> {
        let config_store = self.pool_config_store(block_number, chain).await?;
        self.all_pool_keys_with_config_store(config_store, block_number, chain)
            .await
    }

    async fn all_pool_keys_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<PoolKeyWithAngstromFee>> {
        let all_token_pairs = self
            .all_token_pairs_with_config_store(config_store.clone(), block_number, chain)
            .await?;

        let tokens_to_partial_keys = all_token_pairs
            .into_iter()
            .map(|tokens| {
                (
                    AngstromPoolConfigStore::derive_store_key(tokens.token0, tokens.token1),
                    (tokens.token0, tokens.token1)
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(config_store
            .all_entries()
            .into_iter()
            .map(|entry| {
                let (k, v) = entry.pair();
                let (token0, token1) = tokens_to_partial_keys.get(k).unwrap();

                let pool_key = PoolKey {
                    currency0:   *token0,
                    currency1:   *token1,
                    fee:         U24::from(0x800000),
                    tickSpacing: I24::unchecked_from(v.tick_spacing),
                    hooks:       chain.constants().angstrom_address()
                };
                PoolKeyWithAngstromFee { pool_key, pool_fee_in_e6: U24::from(v.fee_in_e6) }
            })
            .collect())
    }

    async fn pool_id(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<PoolId> {
        self.pool_key_by_tokens(token0, token1, block_number, chain)
            .await
            .map(|key| key.pool_key.into())
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
        block_stream_buffer: Option<usize>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<WithEthMeta<Vec<HistoricalOrders>>>> {
        let bundles = self
            .historical_bundles(filter.from_block, filter.to_block, block_stream_buffer, chain)
            .await?;

        if bundles.is_empty() {
            return Ok(Vec::new())
        }

        let pool_stores =
            AngstromPoolTokenIndexToPair::new_with_tokens(self, &filter, chain).await?;

        Ok(bundles
            .into_iter()
            .map(|bundle| bundle.map_inner(|b| filter.filter_bundle(b, &pool_stores)))
            .collect())
    }

    async fn historical_bundles(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        block_stream_buffer: Option<usize>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<WithEthMeta<AngstromBundle>>>;

    async fn historical_liquidity_changes(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>>;

    async fn historical_post_bundle_unlock_swaps(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::Swap>>>;

    async fn get_bundle_by_block(
        &self,
        block_number: u64,
        verify_successful_tx: bool,
        chain: AngstromL1Chain
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>>;

    async fn get_bundle_by_tx_hash(
        &self,
        tx_hash: TxHash,
        verify_successful_tx: bool
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>>;

    async fn pool_data_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        load_ticks: bool,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<(u64, BaselinePoolStateWithKey)>;

    async fn pool_data_by_pool_id(
        &self,
        pool_id: PoolId,
        load_ticks: bool,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<(u64, BaselinePoolStateWithKey)> {
        let pool_key = self
            .pool_key_by_pool_id(pool_id, block_number, chain)
            .await?;
        self.pool_data_by_tokens(
            pool_key.pool_key.currency0,
            pool_key.pool_key.currency1,
            load_ticks,
            block_number,
            chain
        )
        .await
    }

    async fn all_pool_data(
        &self,
        load_ticks: bool,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<(u64, BaselinePoolStateWithKey)>> {
        let token_pairs = self.all_token_pairs(block_number, chain).await?;

        let pools = futures::future::try_join_all(token_pairs.into_iter().map(|pair| {
            self.pool_data_by_tokens(pair.token0, pair.token1, load_ticks, block_number, chain)
        }))
        .await?;

        Ok(pools)
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<AngstromPoolConfigStore>;

    async fn slot0_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<UnpackedSlot0>;

    async fn slot0_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<UnpackedSlot0> {
        let pool_id = self.pool_id(token0, token1, block_number, chain).await?;
        self.slot0_by_pool_id(pool_id, block_number, chain).await
    }

    async fn fee_configuration_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<FeeConfiguration> {
        let pool_key = self
            .pool_key_by_pool_id(pool_id, block_number, chain)
            .await?;
        self.fee_configuration_by_tokens(
            pool_key.pool_key.currency0,
            pool_key.pool_key.currency1,
            Some(pool_key.pool_fee_in_e6),
            block_number,
            chain
        )
        .await
    }

    async fn fee_configuration_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        bundle_fee: Option<U24>,
        block_number: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<FeeConfiguration>;
}
