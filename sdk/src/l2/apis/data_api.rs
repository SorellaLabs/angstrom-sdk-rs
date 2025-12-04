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
    primitive::{ANGSTROM_ADDRESS, PoolId}
};
use itertools::Itertools;
use uni_v4::FeeConfiguration;
use uniswap_storage::v4::UnpackedSlot0;

use crate::{
    l1::types::*,
    types::{common::*, pool_tick_loaders::PoolTickDataLoader}
};

#[async_trait::async_trait]
pub trait AngstromL2DataApi: PoolTickDataLoader<Ethereum> + Send + Sized {
    async fn all_token_pairs(&self, block_number: Option<u64>) -> eyre::Result<Vec<TokenPair>> {
        let config_store = self.pool_config_store(block_number).await?;
        self.all_token_pairs_with_config_store(config_store, block_number)
            .await
    }

    async fn all_tokens(&self, block_number: Option<u64>) -> eyre::Result<Vec<Address>> {
        let config_store = self.pool_config_store(block_number).await?;
        self.all_tokens_with_config_store(config_store, block_number)
            .await
    }

    async fn all_tokens_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<Address>> {
        Ok(self
            .all_token_pairs_with_config_store(config_store, block_number)
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
        block_number: Option<u64>
    ) -> eyre::Result<PoolKeyWithAngstromFee>;

    async fn pool_key_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>
    ) -> eyre::Result<PoolKeyWithAngstromFee> {
        let config_store = self.pool_config_store(block_number).await?;
        self.pool_key_by_pool_id_with_config_store(pool_id, config_store, block_number)
            .await
    }

    async fn pool_key_by_pool_id_with_config_store(
        &self,
        pool_id: PoolId,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>
    ) -> eyre::Result<PoolKeyWithAngstromFee> {
        self.all_pool_keys_with_config_store(config_store, block_number)
            .await?
            .into_iter()
            .find(|pool_key| pool_id == PoolId::from(pool_key))
            .ok_or_else(|| eyre::eyre!("no pool key for pool_id: {pool_id:?}"))
    }

    async fn all_pool_keys(
        &self,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<PoolKeyWithAngstromFee>> {
        let config_store = self.pool_config_store(block_number).await?;
        self.all_pool_keys_with_config_store(config_store, block_number)
            .await
    }

    async fn pool_id(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<PoolId> {
        self.pool_key_by_tokens(token0, token1, block_number)
            .await
            .map(|key| key.pool_key.into())
    }

    async fn historical_liquidity_changes(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>>;

    async fn pool_data_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        load_ticks: bool,
        block_number: Option<u64>
    ) -> eyre::Result<(u64, BaselinePoolStateWithKey)>;

    async fn pool_data_by_pool_id(
        &self,
        pool_id: PoolId,
        load_ticks: bool,
        block_number: Option<u64>
    ) -> eyre::Result<(u64, BaselinePoolStateWithKey)> {
        let pool_key = self.pool_key_by_pool_id(pool_id, block_number).await?;
        self.pool_data_by_tokens(
            pool_key.pool_key.currency0,
            pool_key.pool_key.currency1,
            load_ticks,
            block_number
        )
        .await
    }

    async fn all_pool_data(
        &self,
        load_ticks: bool,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<(u64, BaselinePoolStateWithKey)>> {
        let token_pairs = self.all_token_pairs(block_number).await?;

        let pools = futures::future::try_join_all(token_pairs.into_iter().map(|pair| {
            self.pool_data_by_tokens(pair.token0, pair.token1, load_ticks, block_number)
        }))
        .await?;

        Ok(pools)
    }

    async fn slot0_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>
    ) -> eyre::Result<UnpackedSlot0>;

    async fn slot0_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<UnpackedSlot0> {
        let pool_id = self.pool_id(token0, token1, block_number).await?;
        self.slot0_by_pool_id(pool_id, block_number).await
    }

    async fn fee_configuration_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>
    ) -> eyre::Result<FeeConfiguration> {
        let pool_key = self.pool_key_by_pool_id(pool_id, block_number).await?;
        self.fee_configuration_by_tokens(
            pool_key.pool_key.currency0,
            pool_key.pool_key.currency1,
            Some(pool_key.pool_fee_in_e6),
            block_number
        )
        .await
    }

    async fn fee_configuration_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        bundle_fee: Option<U24>,
        block_number: Option<u64>
    ) -> eyre::Result<FeeConfiguration>;
}
