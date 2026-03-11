use std::collections::{HashMap, HashSet};

use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::Address;
use alloy_sol_types::SolEvent;
use angstrom_types_primitives::{contract_bindings::pool_manager::PoolManager, primitive::PoolId};
use futures::TryStreamExt;
use op_alloy_network::Optimism;
use uni_v4::{
    BaselinePoolState, L2FeeConfiguration, PoolKey as UniPoolKey,
    baseline_pool_factory::INITIAL_TICKS_PER_SIDE,
    bindings::get_uniswap_v_4_pool_data::GetUniswapV4PoolData,
    liquidity_base::BaselineLiquidity,
    pool_data_loader::{PoolData, PoolDataV4}
};
use uniswap_storage::{
    StorageSlotFetcher,
    angstrom::l2::{
        angstrom_l2::{
            angstrom_l2_jit_tax_enabled, angstrom_l2_pool_fee_config, angstrom_l2_pool_keys_stream,
            angstrom_l2_priority_fee_tax_floor
        },
        angstrom_l2_factory::{
            angstrom_l2_factory_all_hooks, angstrom_l2_factory_get_slot0,
            angstrom_l2_factory_hook_address_for_pool_id
        }
    },
    v4::{UnpackedSlot0, pool_manager::pool_state::pool_manager_pool_slot0}
};

use crate::{
    l2::AngstromL2Chain,
    types::{
        common::*,
        contracts::angstrom_l2::angstrom_l_2_factory::AngstromL2Factory,
        pool_tick_loaders::{DEFAULT_TICKS_PER_BATCH, FullTickLoader, PoolTickDataLoader},
        utils::historical_pool_manager_modify_liquidity_filter
    }
};

impl<P, N> AngstromL2DataApi<N> for P
where
    P: PoolTickDataLoader<N> + StorageSlotFetcher + Send + Sized,
    N: Network
{
}

#[async_trait::async_trait]
pub trait AngstromL2DataApi<N: Network>:
    PoolTickDataLoader<N> + StorageSlotFetcher + Send + Sized
{
    async fn all_pool_keys(
        &self,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<AngstromL2Factory::PoolKey>> {
        let hooks =
            angstrom_l2_factory_all_hooks(self, chain.constants().angstrom_l2_factory(), block_id)
                .await?;

        let pool_key_stream = futures::stream::select_all(
            futures::future::try_join_all(
                hooks
                    .into_iter()
                    .map(|hook| angstrom_l2_pool_keys_stream(self, hook, block_id))
            )
            .await?
            .into_iter()
            .flatten()
        );

        let keys = pool_key_stream
            .map_ok(|pool_key| AngstromL2Factory::PoolKey {
                currency0:   pool_key.currency0,
                currency1:   pool_key.currency1,
                fee:         pool_key.fee,
                tickSpacing: pool_key.tickSpacing,
                hooks:       pool_key.hooks
            })
            .try_collect()
            .await?;

        Ok(keys)
    }

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
            .find(|key| PoolId::from(key) == pool_id)
            .ok_or_else(|| eyre::eyre!("no pool key found for pool id '{pool_id:?}'"))
    }

    async fn historical_liquidity_changes(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>> {
        let all_pool_ids = self
            .all_pool_keys(end_block.map(Into::into).unwrap_or_else(BlockId::latest), chain)
            .await?
            .into_iter()
            .map(PoolId::from)
            .collect::<HashSet<_>>();

        let consts = chain.constants();
        let filters = historical_pool_manager_modify_liquidity_filter(
            start_block,
            end_block,
            consts.uniswap_constants().pool_manager(),
            consts.angstrom_deploy_block()
        );

        let logs = futures::future::try_join_all(
            filters
                .into_iter()
                .map(async move |filter| self.fetch_logs(&filter).await)
        )
        .await?;

        Ok(logs
            .into_iter()
            .flatten()
            .flat_map(|log| {
                PoolManager::ModifyLiquidity::decode_log(&log.inner)
                    .ok()
                    .and_then(|inner_log| {
                        all_pool_ids.contains(&inner_log.id).then(|| {
                            WithEthMeta::new(
                                log.block_number,
                                log.transaction_hash,
                                log.transaction_index,
                                None,
                                inner_log.data
                            )
                        })
                    })
            })
            .collect())
    }

    async fn pool_data_by_pool_id(
        &self,
        pool_id: PoolId,
        load_ticks: bool,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<(u64, BaselinePoolStateWithKey<Optimism>)> {
        let pool_key = self.pool_key_by_pool_id(pool_id, block_id, chain).await?;

        let uni_pool_key = UniPoolKey {
            currency0:   pool_key.currency0,
            currency1:   pool_key.currency1,
            fee:         pool_key.fee,
            tickSpacing: pool_key.tickSpacing,
            hooks:       pool_key.hooks
        };

        let pool_id: PoolId = pool_key.into();

        let data_deployer_call = GetUniswapV4PoolData::deploy_builder(
            self.alloy_root_provider().await?,
            pool_id,
            chain.constants().uniswap_constants().pool_manager(),
            pool_key.currency0,
            pool_key.currency1
        )
        .into_transaction_request();

        let out_pool_data = self
            .view_deploy_call::<PoolDataV4>(block_id, data_deployer_call)
            .await?;
        let pool_data: PoolData = (uni_pool_key, out_pool_data).into();

        let fee_config = self
            .fee_configuration_by_pool_id(pool_id, block_id, chain)
            .await?;

        let (ticks, tick_bitmap) = if load_ticks {
            self.load_tick_data_in_band(
                pool_id,
                pool_data.tick.as_i32(),
                uni_pool_key.tickSpacing.as_i32(),
                block_id,
                INITIAL_TICKS_PER_SIDE,
                DEFAULT_TICKS_PER_BATCH,
                chain.constants().uniswap_constants().pool_manager()
            )
            .await?
        } else {
            (HashMap::default(), HashMap::default())
        };

        let liquidity = pool_data.liquidity;
        let sqrt_price_x96 = pool_data.sqrtPrice.into();
        let tick = pool_data.tick.as_i32();
        let tick_spacing = pool_data.tickSpacing.as_i32();

        let block_number = self.block_number_from_block_id(block_id).await?;

        let baseline_liquidity = BaselineLiquidity::new(
            tick_spacing,
            tick,
            sqrt_price_x96,
            liquidity,
            ticks,
            tick_bitmap
        );

        let baseline_state = BaselinePoolState::new(
            baseline_liquidity,
            block_number,
            fee_config,
            pool_data.tokenA,
            pool_data.tokenB,
            pool_data.tokenADecimals,
            pool_data.tokenBDecimals
        );

        Ok((
            block_number,
            BaselinePoolStateWithKey {
                pool:     baseline_state,
                pool_key: PoolManager::PoolKey {
                    currency0:   pool_key.currency0,
                    currency1:   pool_key.currency1,
                    fee:         pool_key.fee,
                    tickSpacing: pool_key.tickSpacing,
                    hooks:       pool_key.hooks
                }
            }
        ))
    }

    async fn all_pool_data(
        &self,
        load_ticks: bool,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<(u64, BaselinePoolStateWithKey<Optimism>)>> {
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
    ) -> eyre::Result<UnpackedSlot0> {
        Ok(pool_manager_pool_slot0(
            self,
            chain.constants().uniswap_constants().pool_manager(),
            pool_id,
            block_id
        )
        .await?)
    }

    async fn hook_by_pool_id(
        &self,
        pool_id: PoolId,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<Address> {
        Ok(angstrom_l2_factory_hook_address_for_pool_id(
            self,
            chain.constants().angstrom_l2_factory(),
            pool_id,
            block_id
        )
        .await?
        .ok_or_else(|| eyre::eyre!("no hook found for pool id: {pool_id:?}"))?)
    }

    async fn fee_configuration_by_pool_id(
        &self,
        pool_id: PoolId,
        block_id: BlockId,
        chain: AngstromL2Chain
    ) -> eyre::Result<L2FeeConfiguration> {
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
    ) -> eyre::Result<L2FeeConfiguration> {
        let (fee_config, priority_fee_tax_floor, jit_tax_enabled, factory_slot0, pool_key) = tokio::try_join!(
            angstrom_l2_pool_fee_config(self, hook_address, pool_id, block_id),
            angstrom_l2_priority_fee_tax_floor(self, hook_address, block_id),
            angstrom_l2_jit_tax_enabled(self, hook_address, block_id),
            angstrom_l2_factory_get_slot0(self, hook_address, block_id),
            self.pool_key_by_pool_id(pool_id, block_id, chain)
        )?;

        Ok(L2FeeConfiguration {
            is_initialized: fee_config.is_initialized,
            lp_fee: pool_key.fee.to(),
            creator_tax_fee_e6: fee_config.creator_tax_fee_e6,
            protocol_tax_fee_e6: fee_config.protocol_tax_fee_e6,
            creator_swap_fee_e6: fee_config.creator_swap_fee_e6,
            protocol_swap_fee_e6: fee_config.protocol_swap_fee_e6,
            priority_fee_tax_floor: priority_fee_tax_floor.to(),
            jit_tax_enabled,
            withdraw_only: factory_slot0.withdraw_only
        })
    }
}

#[cfg(test)]
mod data_api_tests {

    use super::*;
    use crate::l2::test_utils::{
        BASE_USDC, valid_test_params::init_valid_position_params_with_provider
    };

    #[tokio::test]
    async fn test_fetch_fee_configuration() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let fee_config = provider
            .fee_configuration_by_pool_id_and_hook(
                state.pool_id,
                state.pool_key.hooks,
                state.block_number.into(),
                state.chain
            )
            .await
            .unwrap();

        let expected = L2FeeConfiguration {
            is_initialized:         true,
            lp_fee:                 160,
            creator_tax_fee_e6:     Default::default(),
            protocol_tax_fee_e6:    Default::default(),
            creator_swap_fee_e6:    Default::default(),
            protocol_swap_fee_e6:   35,
            priority_fee_tax_floor: 10000000,
            jit_tax_enabled:        false,
            withdraw_only:          false
        };

        assert_eq!(expected, fee_config);
    }

    #[tokio::test]
    async fn test_all_token_pairs() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let all_pairs = provider
            .all_token_pairs(state.block_number.into(), state.chain)
            .await
            .unwrap();

        assert_eq!(all_pairs.len(), 3);
        assert!(all_pairs.contains(&TokenPair { token0: Address::ZERO, token1: BASE_USDC }));
    }

    #[tokio::test]
    async fn test_all_tokens() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let all_tokens = provider
            .all_tokens(state.block_number.into(), state.chain)
            .await
            .unwrap();

        assert_eq!(all_tokens.len(), 6);
    }

    #[tokio::test]
    async fn test_pool_key_by_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let pool_key = provider
            .pool_key_by_pool_id(state.pool_id, state.block_number.into(), state.chain)
            .await
            .unwrap();

        assert_eq!(PoolId::from(pool_key), PoolId::from(state.pool_key));
    }

    #[tokio::test]
    async fn test_historical_liquidity_changes() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let modify_liquidity = provider
            .historical_liquidity_changes(
                Some(state.block_for_liquidity_add),
                Some(state.block_for_liquidity_add),
                state.chain
            )
            .await
            .unwrap();

        assert_eq!(modify_liquidity.len(), 1);
    }

    #[tokio::test]
    async fn test_pool_data_by_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let (_, pool_data) = provider
            .pool_data_by_pool_id(
                PoolId::from(state.pool_key),
                true,
                state.block_number.into(),
                state.chain
            )
            .await
            .unwrap();

        assert_eq!(pool_data.pool.token0, state.pool_key.currency0);
        assert_eq!(pool_data.pool.token1, state.pool_key.currency1);
        assert!(
            !pool_data
                .pool
                .get_baseline_liquidity()
                .initialized_ticks()
                .is_empty()
        );
    }

    #[tokio::test]
    async fn test_all_pool_data() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let all_pool_data = provider
            .all_pool_data(true, state.block_number.into(), state.chain)
            .await
            .unwrap();

        assert_eq!(all_pool_data.len(), 3);
    }

    #[tokio::test]
    async fn test_slot0_by_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let slot0 = provider
            .slot0_by_pool_id(PoolId::from(state.pool_key), state.block_number.into(), state.chain)
            .await
            .unwrap();

        assert_eq!(slot0.tick, state.current_pool_tick);
    }
}
