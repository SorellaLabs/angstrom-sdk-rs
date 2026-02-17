use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::Address;
use angstrom_types_primitives::{contract_bindings::pool_manager::PoolManager, primitive::PoolId};
use uniswap_storage::{angstrom::l2::AngstromL2PoolFeeConfiguration, v4::UnpackedSlot0};

use crate::{
    l2::AngstromL2Chain,
    types::{
        common::*, contracts::angstrom_l2::angstrom_l_2_factory::AngstromL2Factory,
        pool_tick_loaders::PoolTickDataLoader
    }
};

#[async_trait::async_trait]
pub trait AngstromL2DataApi<N: Network>: PoolTickDataLoader<N> + Send + Sized {
    async fn all_pool_keys(
        &self,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<AngstromL2Factory::PoolKey>>;

    async fn all_token_pairs(
        &self,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<TokenPair>> {
        Ok(self
            .all_pool_keys(block_id, chain)
            .await?
            .into_iter()
            .map(|key| TokenPair { token0: key.currency0, token1: key.currency1 })
            .collect())
    }

    async fn pool_keys_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<AngstromL2Factory::PoolKey>> {
        Ok(self
            .all_pool_keys(block_id, chain)
            .await?
            .into_iter()
            .filter(|key| key.currency0 == token0 && key.currency1 == token1)
            .collect())
    }

    async fn all_tokens(
        &self,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<Address>> {
        Ok(self
            .all_pool_keys(block_id, chain)
            .await?
            .into_iter()
            .flat_map(|key| [key.currency0, key.currency1])
            .collect())
    }

    async fn pool_key_by_pool_id(
        &self,
        pool_id: PoolId,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<AngstromL2Factory::PoolKey> {
        self.all_pool_keys(block_id, chain)
            .await?
            .into_iter()
            .filter(|key| PoolId::from(key) == pool_id)
            .next()
            .ok_or_else(|| eyre::eyre!("no pool key found for pool id '{pool_id:?}'"))
    }

    async fn historical_liquidity_changes(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>>;

    async fn pool_data_by_pool_id(
        &self,
        pool_id: PoolId,
        load_ticks: bool,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<(u64, BaselinePoolStateWithKey)>;

    async fn all_pool_data(
        &self,
        load_ticks: bool,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<(u64, BaselinePoolStateWithKey)>> {
        let pool_ids = self
            .all_pool_keys(block_id, chain)
            .await?
            .into_iter()
            .map(Into::into)
            .collect::<Vec<PoolId>>();

        let pools = futures::future::try_join_all(
            pool_ids
                .into_iter()
                .map(|pool_id| self.pool_data_by_pool_id(pool_id, load_ticks, block_id, chain))
        )
        .await?;

        Ok(pools)
    }

    async fn slot0_by_pool_id(
        &self,
        pool_id: PoolId,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<UnpackedSlot0>;

    async fn hook_by_pool_id(
        &self,
        pool_id: PoolId,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Address>;

    async fn fee_configuration_by_pool_id(
        &self,
        pool_id: PoolId,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<AngstromL2PoolFeeConfiguration> {
        let hook = self.hook_by_pool_id(pool_id, block_id, chain).await?;
        Ok(self
            .fee_configuration_by_pool_id_and_hook(pool_id, hook, block_id, chain)
            .await?)
    }

    async fn fee_configuration_by_pool_id_and_hook(
        &self,
        pool_id: PoolId,
        hook_address: Address,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<AngstromL2PoolFeeConfiguration>;
}
