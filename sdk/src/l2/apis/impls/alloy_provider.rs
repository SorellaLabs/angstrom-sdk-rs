use std::collections::{HashMap, HashSet};

use alloy_network::Network;
use alloy_primitives::{Address, B256, U256, aliases::I24};
use alloy_provider::Provider;
use alloy_rpc_types::Filter;
use alloy_sol_types::SolEvent;
use angstrom_types_primitives::{
    contract_bindings::pool_manager::PoolManager::{self, PoolKey},
    primitive::PoolId
};
use uni_v4::{
    BaselinePoolState, FeeConfiguration, PoolKey as UniPoolKey,
    baseline_pool_factory::INITIAL_TICKS_PER_SIDE,
    liquidity_base::BaselineLiquidity,
    loaders::get_uniswap_v_4_pool_data::GetUniswapV4PoolData,
    pool_data_loader::{PoolData, PoolDataV4}
};
use uniswap_storage::{
    angstrom::l2::{
        AngstromL2PoolFeeConfiguration,
        angstrom_l2::{
            angstrom_l2_growth_inside, angstrom_l2_last_growth_inside, angstrom_l2_pool_fee_config
        },
        angstrom_l2_factory::angstrom_l2_factory_hook_address_for_pool_id
    },
    v4::{
        UnpackedPositionInfo, UnpackedSlot0, V4UserLiquidityPosition,
        pool_manager::{
            pool_state::pool_manager_pool_slot0,
            position_state::pool_manager_position_state_liquidity
        },
        position_manager::{
            position_manager_next_token_id, position_manager_owner_of,
            position_manager_pool_key_and_info
        }
    }
};

use crate::{
    l2::{
        AngstromL2Chain,
        apis::{AngstromL2DataApi, AngstromL2UserApi}
    },
    types::{
        common::*,
        contracts::angstrom_l2::angstrom_l_2_factory::AngstromL2Factory,
        fees::{LiquidityPositionFees, uniswap_fee_deltas},
        pool_tick_loaders::{DEFAULT_TICKS_PER_BATCH, FullTickLoader},
        providers::{AlloyProviderWrapper, alloy_view_deploy},
        utils::historical_pool_manager_modify_liquidity_filter
    }
};

#[async_trait::async_trait]
impl<P, N> AngstromL2DataApi<N> for AlloyProviderWrapper< N>
where
    P: Provider<N> + Clone + Sync,
    N: Network
{
    async fn all_pool_keys(
        &self,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<AngstromL2Factory::PoolKey>> {
        let constants = chain.constants();

        let mut filter = Filter::new()
            .address(constants.angstrom_l2_factory())
            .from_block(constants.angstrom_deploy_block())
            .event_signature(AngstromL2Factory::PoolCreated::SIGNATURE_HASH);

        if let Some(bn) = block_number {
            filter = filter.to_block(bn);
        }

        let keys = self
            .provider()
            .get_logs(&filter)
            .await?
            .into_iter()
            .flat_map(|raw_log| {
                AngstromL2Factory::PoolCreated::decode_log(&raw_log.inner)
                    .map(|log| log.key)
                    .ok()
            })
            .collect();

        Ok(keys)
    }

    async fn pool_data_by_pool_id(
        &self,
        pool_id: PoolId,
        load_ticks: bool,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<(u64, BaselinePoolStateWithKey)> {
        let block_number = match block_number {
            Some(bn) => bn,
            None => self.provider().get_block_number().await?
        };

        let pool_key = self
            .pool_key_by_pool_id(pool_id, Some(block_number), chain)
            .await?;

        let uni_pool_key = UniPoolKey {
            currency0:   pool_key.currency0,
            currency1:   pool_key.currency1,
            fee:         pool_key.fee,
            tickSpacing: pool_key.tickSpacing,
            hooks:       pool_key.hooks
        };

        let pool_id: PoolId = pool_key.into();

        let data_deployer_call = GetUniswapV4PoolData::deploy_builder(
            self.provider(),
            pool_id,
            chain.constants().uniswap_constants().pool_manager(),
            pool_key.currency0,
            pool_key.currency1
        )
        .into_transaction_request();

        let out_pool_data = alloy_view_deploy::<_, _, PoolDataV4>(
            self.provider(),
            Some(block_number),
            data_deployer_call
        )
        .await??;
        let pool_data: PoolData = (uni_pool_key, out_pool_data).into();

        // let fee_config = self
        //     .fee_configuration_by_pool_id(pool_id, Some(block_number), chain)
        //     .await?;
        let fee_config = FeeConfiguration {
            bundle_fee:   Default::default(),
            swap_fee:     Default::default(),
            protocol_fee: Default::default()
        };

        let (ticks, tick_bitmap) = if load_ticks {
            self.load_tick_data_in_band(
                pool_id,
                pool_data.tick.as_i32(),
                uni_pool_key.tickSpacing.as_i32(),
                Some(block_number),
                INITIAL_TICKS_PER_SIDE,
                DEFAULT_TICKS_PER_BATCH
            )
            .await?
        } else {
            (HashMap::default(), HashMap::default())
        };

        let liquidity = pool_data.liquidity;
        let sqrt_price_x96 = pool_data.sqrtPrice.into();
        let tick = pool_data.tick.as_i32();
        let tick_spacing = pool_data.tickSpacing.as_i32();

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

    async fn historical_liquidity_changes(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>> {
        let all_pool_ids = self
            .all_pool_keys(end_block, chain)
            .await?
            .into_iter()
            .map(|pool_key| PoolId::from(pool_key))
            .collect::<HashSet<_>>();

        let filters = historical_pool_manager_modify_liquidity_filter(start_block, end_block);
        let provider = self.provider();

        let logs = futures::future::try_join_all(
            filters
                .into_iter()
                .map(async move |filter| provider.get_logs(&filter).await)
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
                                inner_log.data
                            )
                        })
                    })
            })
            .collect())
    }

    async fn slot0_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<UnpackedSlot0> {
        Ok(pool_manager_pool_slot0(
            self.root(),
            chain.constants().uniswap_constants().pool_manager(),
            pool_id,
            block_number.map(Into::into)
        )
        .await?)
    }

    async fn hook_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<Address> {
        Ok(angstrom_l2_factory_hook_address_for_pool_id(
            self.root(),
            chain.constants().angstrom_l2_factory(),
            pool_id,
            block_number.map(Into::into)
        )
        .await?
        .ok_or_else(|| eyre::eyre!("no hook found for pool id: {pool_id:?}"))?)
    }

    async fn fee_configuration_by_pool_id_and_hook(
        &self,
        pool_id: PoolId,
        hook_address: Address,
        block_number: Option<u64>,
        _chain: AngstromL2Chain
    ) -> eyre::Result<AngstromL2PoolFeeConfiguration> {
        angstrom_l2_pool_fee_config(self.root(), hook_address, pool_id, block_number.map(Into::into)).await
    }
}

#[async_trait::async_trait]
impl<P, N> AngstromL2UserApi<N> for AlloyProviderWrapper< N>
where
    P: Provider<N> + Clone + Sync,
    N: Network
{
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)> {
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            self.root(),
            chain.constants().uniswap_constants().position_manager(),
            block_number.map(Into::into),
            position_token_id
        )
        .await?;

        Ok((
            PoolKey {
                currency0:   pool_key.currency0,
                currency1:   pool_key.currency1,
                fee:         pool_key.fee,
                tickSpacing: pool_key.tickSpacing,
                hooks:       pool_key.hooks
            },
            position_info
        ))
    }

    async fn position_liquidity(
        &self,
        position_token_id: U256,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<u128> {
        let consts = chain.constants();
        let block_id = block_number.map(Into::into);
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            self.root(),
            consts.uniswap_constants().position_manager(),
            block_id,
            position_token_id
        )
        .await?;

        let liquidity = pool_manager_position_state_liquidity(
            self.root(),
            consts.uniswap_constants().pool_manager(),
            consts.uniswap_constants().position_manager(),
            pool_key.into(),
            position_token_id,
            position_info.tick_lower,
            position_info.tick_upper,
            block_id
        )
        .await?;

        Ok(liquidity)
    }

    async fn all_user_positions(
        &self,
        owner: Address,
        mut start_token_id: U256,
        mut end_token_id: U256,
        pool_id: Option<PoolId>,
        max_results: Option<usize>,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<Vec<V4UserLiquidityPosition>> {
        let consts = chain.constants();

        let position_manager_address = consts.uniswap_constants().position_manager();
        let pool_manager_address = consts.uniswap_constants().pool_manager();
        let block_id = block_number.map(Into::into);

        let all_angstrom_hooks = if pool_id.is_none() {
            self.all_pool_keys(block_number, chain)
                .await?
                .into_iter()
                .map(|key| key.hooks)
                .collect::<HashSet<_>>()
        } else {
            HashSet::new()
        };

        let root = self.root();

        if start_token_id == U256::ZERO {
            start_token_id = U256::from(1u8);
        }

        if end_token_id == U256::ZERO {
            end_token_id =
                position_manager_next_token_id(root, position_manager_address, block_id)
                    .await?;
        }

        let mut all_positions = Vec::new();
        while start_token_id <= end_token_id {
            let owner_of = position_manager_owner_of(
                root,
                position_manager_address,
                block_id,
                start_token_id
            )
            .await?;

            if owner_of != owner {
                start_token_id += U256::from(1u8);
                continue;
            }

            let (pool_key, position_info) = position_manager_pool_key_and_info(
                root,
                position_manager_address,
                block_id,
                start_token_id
            )
            .await?;

            if !all_angstrom_hooks.contains(&pool_key.hooks)
                || pool_id
                    .map(|id| id != B256::from(pool_key))
                    .unwrap_or_default()
            {
                start_token_id += U256::from(1u8);
                continue;
            }

            let liquidity = pool_manager_position_state_liquidity(
                root,
                pool_manager_address,
                position_manager_address,
                pool_key.into(),
                start_token_id,
                position_info.tick_lower,
                position_info.tick_upper,
                block_id
            )
            .await?;

            all_positions.push(V4UserLiquidityPosition {
                token_id: start_token_id,
                tick_lower: position_info.tick_lower,
                tick_upper: position_info.tick_upper,
                liquidity,
                pool_key
            });

            if let Some(max_res) = max_results {
                if all_positions.len() >= max_res {
                    break;
                }
            }

            start_token_id += U256::from(1u8);
        }

        Ok(all_positions)
    }

    async fn user_position_fees(
        &self,
        position_token_id: U256,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<LiquidityPositionFees> {
        let consts = chain.constants();
        let block_id = block_number.map(Into::into);

        let ((pool_key, position_info), position_liquidity) = tokio::try_join!(
            self.position_and_pool_info(position_token_id, block_number, chain),
            self.position_liquidity(position_token_id, block_number, chain),
        )?;

        let hook = pool_key.hooks;
        let pool_id = pool_key.into();
        let slot0 = self.slot0_by_pool_id(pool_id, block_number, chain).await?;

        let (angstrom_fee_delta, (uniswap_token0_fee_delta, uniswap_token1_fee_delta)) = tokio::try_join!(
            self.angstrom_l2_fees(
                pool_id,
                Some(hook),
                slot0.tick,
                position_token_id,
                position_info.tick_lower,
                position_info.tick_upper,
                block_number,
                chain
            ),
            uniswap_fee_deltas(
                self.root(),
                consts.uniswap_constants().pool_manager(),
                consts.uniswap_constants().position_manager(),
                block_id,
                pool_id,
                slot0.tick,
                position_token_id,
                position_info.tick_lower,
                position_info.tick_upper,
            )
        )?;

        Ok(LiquidityPositionFees::new(
            position_liquidity,
            angstrom_fee_delta,
            uniswap_token0_fee_delta,
            uniswap_token1_fee_delta
        ))
    }

    async fn angstrom_l2_fees(
        &self,
        pool_id: PoolId,
        hook_address: Option<Address>,
        current_pool_tick: I24,
        position_token_id: U256,
        tick_lower: I24,
        tick_upper: I24,
        block_number: Option<u64>,
        chain: AngstromL2Chain
    ) -> eyre::Result<U256> {
        let hook = if let Some(hook_address) = hook_address {
            hook_address
        } else {
            self.hook_by_pool_id(pool_id, block_number, chain).await?
        };
        let consts = chain.constants();
        let block_id = block_number.map(Into::into);
        let (growth_inside, last_growth_inside) = tokio::try_join!(
            angstrom_l2_growth_inside(
                self.root(),
                hook,
                pool_id,
                current_pool_tick,
                tick_lower,
                tick_upper,
                block_id,
            ),
            angstrom_l2_last_growth_inside(
                self.root(),
                hook,
                consts.uniswap_constants().position_manager(),
                pool_id,
                position_token_id,
                tick_lower,
                tick_upper,
                block_id,
            ),
        )?;

        Ok(growth_inside - last_growth_inside)
    }
}

#[cfg(test)]
mod data_api_tests {

    use super::*;
    use crate::l2::test_utils::valid_test_params::init_valid_position_params_with_provider;

    // #[tokio::test]
    // async fn test_fetch_fee_configuration() {
    //     let (provider, state) = init_valid_position_params_with_provider().await;

    //     let fee_config = provider
    //         .fee_configuration_by_pool_id(state.pool_id,
    // Some(state.block_number), state.chain)         .await
    //         .unwrap();

    //     assert_eq!(
    //         FeeConfiguration { bundle_fee: 200, swap_fee: 238, protocol_fee: 112
    // },         fee_config
    //     );
    // }

    #[tokio::test]
    async fn test_all_token_pairs() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let all_pairs = provider
            .all_token_pairs(Some(state.block_number), state.chain)
            .await
            .unwrap();

        assert_eq!(all_pairs.len(), 2);
        assert!(!all_pairs.contains(&TokenPair { token0: Address::ZERO, token1: Address::ZERO }));
    }

    #[tokio::test]
    async fn test_all_tokens() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let all_tokens = provider
            .all_tokens(Some(state.block_number), state.chain)
            .await
            .unwrap();

        assert_eq!(all_tokens.len(), 3);
        assert!(!all_tokens.contains(&Address::ZERO));
    }

    #[tokio::test]
    async fn test_pool_key_by_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let pool_key = provider
            .pool_key_by_pool_id(state.pool_key.into(), Some(state.block_number), state.chain)
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
    async fn test_pool_data_by_tokens() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let (_, pool_data) = provider
            .pool_data_by_pool_id(state.pool_id, true, Some(state.block_number), state.chain)
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
    async fn test_pool_data_by_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let (_, pool_data) = provider
            .pool_data_by_pool_id(
                PoolId::from(state.pool_key),
                true,
                Some(state.block_number),
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
            .all_pool_data(true, Some(state.block_number), state.chain)
            .await
            .unwrap();

        assert_eq!(all_pool_data.len(), 2);
    }

    #[tokio::test]
    async fn test_slot0_by_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let slot0 = provider
            .slot0_by_pool_id(PoolId::from(state.pool_key), Some(state.block_number), state.chain)
            .await
            .unwrap();

        assert_eq!(slot0.tick, state.current_pool_tick);
    }
}

#[cfg(test)]
mod user_api_tests {

    use alloy_primitives::U256;

    use crate::{
        l2::{
            apis::user_api::AngstromL2UserApi,
            test_utils::valid_test_params::init_valid_position_params_with_provider
        },
        types::fees::LiquidityPositionFees
    };

    #[tokio::test]
    async fn test_position_and_pool_info_by_token_id() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_for_liquidity_add + 1;

        let (pool_key, unpacked_position_info) = provider
            .position_and_pool_info(pos_info.position_token_id, Some(block_number), pos_info.chain)
            .await
            .unwrap();

        assert_eq!(pool_key, pos_info.pool_key);
        assert_eq!(unpacked_position_info, pos_info.as_unpacked_position_info());
    }

    #[tokio::test]
    async fn test_position_liquidity_by_token_id() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_for_liquidity_add + 1;

        let position_liquidity = provider
            .position_liquidity(pos_info.position_token_id, Some(block_number), pos_info.chain)
            .await
            .unwrap();

        assert_eq!(pos_info.position_liquidity, position_liquidity);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_all_user_positions() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_for_liquidity_add + 1;

        let bound: u64 = 10;

        let position_liquidity = provider
            .all_user_positions(
                pos_info.owner,
                pos_info.position_token_id - U256::from(bound),
                pos_info.position_token_id + U256::from(bound),
                None,
                None,
                Some(block_number),
                pos_info.chain
            )
            .await
            .unwrap();

        assert_eq!(position_liquidity.len(), 1);
    }

    #[tokio::test]
    async fn test_user_position_fees() {
        let (provider, pos_info) = init_valid_position_params_with_provider().await;
        let block_number = pos_info.block_for_liquidity_add + 100;

        let results = provider
            .user_position_fees(pos_info.position_token_id, Some(block_number), pos_info.chain)
            .await
            .unwrap();

        assert_eq!(
            results,
            LiquidityPositionFees {
                position_liquidity:   807449445327074,
                angstrom_token0_fees: U256::from(45197_u128),
                uniswap_token0_fees:  U256::from(2754_u128),
                uniswap_token1_fees:  U256::from(837588354352_u128)
            }
        );
    }
}
