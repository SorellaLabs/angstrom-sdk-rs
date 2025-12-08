use std::{
    collections::{HashMap, HashSet},
    ops::Deref
};

use alloy_consensus::Transaction;
use alloy_eips::BlockId;
use alloy_network::Network;
use alloy_primitives::{
    Address, B256, FixedBytes, TxHash, U256,
    aliases::{I24, U24},
    keccak256
};
use alloy_provider::{DynProvider, Provider};
use alloy_rpc_types::Filter;
use alloy_sol_types::{SolCall, SolEvent};
use angstrom_types_primitives::{
    contract_bindings::{
        angstrom::Angstrom,
        controller_v_1::ControllerV1::getPoolByKeyCall,
        pool_manager::PoolManager::{self, PoolKey}
    },
    contract_payloads::angstrom::{
        AngstromBundle, AngstromPoolConfigStore, AngstromPoolPartialKey
    },
    primitive::{
        ANGSTROM_ADDRESS, CONTROLLER_V1_ADDRESS, POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS,
        PoolId
    }
};
use futures::StreamExt;
use pade::PadeDecode;
use uni_v4::{
    BaselinePoolState, FeeConfiguration, PoolKey as UniPoolKey,
    baseline_pool_factory::INITIAL_TICKS_PER_SIDE,
    liquidity_base::BaselineLiquidity,
    loaders::get_uniswap_v_4_pool_data::GetUniswapV4PoolData,
    pool_data_loader::{PoolData, PoolDataV4}
};
use uniswap_storage::v4::{
    UnpackedPositionInfo, UnpackedSlot0, V4UserLiquidityPosition,
    pool_manager::{
        pool_state::pool_manager_pool_slot0, position_state::pool_manager_position_state_liquidity
    },
    position_manager::{
        position_manager_next_token_id, position_manager_owner_of,
        position_manager_pool_key_and_info
    }
};

use crate::{
    l2::{
        apis::{AngstromL2DataApi, AngstromL2UserApi},
        constants::AngstromL2Chain
    },
    types::{
        common::*,
        contracts::angstrom_l2::{
            angstrom_l_2::AngstromL2, angstrom_l_2_factory::AngstromL2Factory
        },
        fees::{LiquidityPositionFees, position_fees},
        pool_tick_loaders::{DEFAULT_TICKS_PER_BATCH, FullTickLoader},
        providers::{alloy_view_call, alloy_view_deploy},
        utils::{
            historical_pool_manager_modify_liquidity_filter, historical_pool_manager_swap_filter
        }
    }
};

#[async_trait::async_trait]
impl<P: Provider<N> + Clone, N: Network> AngstromL2DataApi<N> for P {
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
}

/*
=

    async fn historical_liquidity_changes(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>> {
        let all_pool_ids = self
            .all_pool_keys(end_block)
            .await?
            .into_iter()
            .map(|val| PoolId::from(val.pool_key))
            .collect::<HashSet<_>>();

        let filters = historical_pool_manager_modify_liquidity_filter(start_block, end_block);

        let logs = futures::future::try_join_all(
            filters
                .into_iter()
                .map(async move |filter| self.get_logs(&filter).await)
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

    async fn pool_data_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        load_ticks: bool,
        block_number: Option<u64>
    ) -> eyre::Result<(u64, BaselinePoolStateWithKey)> {
        let block_number = match block_number {
            Some(bn) => bn,
            None => self.get_block_number().await?
        };

        let (token0, token1) = sort_tokens(token0, token1);

        let pool_key = self
            .pool_key_by_tokens(token0, token1, Some(block_number))
            .await?;

        let uni_pool_key = UniPoolKey {
            currency0:   pool_key.pool_key.currency0,
            currency1:   pool_key.pool_key.currency1,
            fee:         pool_key.pool_fee_in_e6,
            tickSpacing: pool_key.pool_key.tickSpacing,
            hooks:       pool_key.pool_key.hooks
        };

        let pool_id: PoolId = pool_key.into();

        let data_deployer_call = GetUniswapV4PoolData::deploy_builder(
            &self,
            pool_id,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            pool_key.pool_key.currency0,
            pool_key.pool_key.currency1
        )
        .into_transaction_request();

        let out_pool_data =
            alloy_view_deploy::<_, _, PoolDataV4>(&self, Some(block_number), data_deployer_call)
                .await??;
        let pool_data: PoolData = (uni_pool_key, out_pool_data).into();

        let fee_config = self
            .fee_configuration_by_tokens(
                pool_key.pool_key.currency0,
                pool_key.pool_key.currency1,
                Some(pool_key.pool_fee_in_e6),
                Some(block_number)
            )
            .await?;

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
            BaselinePoolStateWithKey { pool: baseline_state, pool_key: pool_key.pool_key }
        ))
    }

    async fn slot0_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>
    ) -> eyre::Result<UnpackedSlot0> {
        Ok(pool_manager_pool_slot0(
            self.root(),
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            pool_id,
            block_number
        )
        .await?)
    }

    async fn fee_configuration_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        bundle_fee: Option<U24>,
        block_number: Option<u64>
    ) -> eyre::Result<FeeConfiguration> {
        const UNLOCKED_FEES_SLOT: u64 = 2;

        let pool_partial_key = AngstromPoolConfigStore::derive_store_key(token0, token1);

        let mut preimage = [0u8; 64];
        let key_bytes: &[u8; 27] = pool_partial_key.deref();
        preimage[..27].copy_from_slice(key_bytes);
        preimage[32..].copy_from_slice(&U256::from(UNLOCKED_FEES_SLOT).to_be_bytes::<32>());
        let slot = keccak256(preimage);

        let bundle_fee = if let Some(f) = bundle_fee {
            f
        } else {
            self.pool_key_by_tokens(token0, token1, block_number)
                .await?
                .pool_fee_in_e6
        };

        let raw = alloy_view_call(
            &self,
            block_number,
            *ANGSTROM_ADDRESS.get().unwrap(),
            Angstrom::extsloadCall { slot: U256::from_be_bytes(*slot) }
        )
        .await??;

        let bytes = raw.to_be_bytes::<32>();
        let unlocked_fee = U24::from_be_bytes([bytes[29], bytes[30], bytes[31]]);
        let protocol_fee = U24::from_be_bytes([bytes[26], bytes[27], bytes[28]]);

        Ok(FeeConfiguration {
            bundle_fee:   bundle_fee.to::<u32>(),
            swap_fee:     unlocked_fee.to::<u32>(),
            protocol_fee: protocol_fee.to::<u32>()
        })
    }
}

#[async_trait::async_trait]
impl<P: Provider + Clone> AngstromL2UserApi for P {
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)> {
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            self.root(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            block_number,
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
        block_number: Option<u64>
    ) -> eyre::Result<u128> {
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            self.root(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            block_number,
            position_token_id
        )
        .await?;

        let liquidity = pool_manager_position_state_liquidity(
            self.root(),
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            pool_key.into(),
            position_token_id,
            position_info.tick_lower,
            position_info.tick_upper,
            block_number
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
        block_number: Option<u64>
    ) -> eyre::Result<Vec<V4UserLiquidityPosition>> {
        let position_manager_address = *POSITION_MANAGER_ADDRESS.get().unwrap();
        let pool_manager_address = *POOL_MANAGER_ADDRESS.get().unwrap();
        let angstrom_address = *ANGSTROM_ADDRESS.get().unwrap();

        let root = self.root();

        if start_token_id == U256::ZERO {
            start_token_id = U256::from(1u8);
        }

        if end_token_id == U256::ZERO {
            end_token_id =
                position_manager_next_token_id(root, position_manager_address, block_number)
                    .await?;
        }

        let mut all_positions = Vec::new();
        while start_token_id <= end_token_id {
            let owner_of = position_manager_owner_of(
                root,
                position_manager_address,
                block_number,
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
                block_number,
                start_token_id
            )
            .await?;

            if pool_key.hooks != angstrom_address
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
                block_number
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
        block_number: Option<u64>
    ) -> eyre::Result<LiquidityPositionFees> {
        let ((pool_key, position_info), position_liquidity) = tokio::try_join!(
            self.position_and_pool_info(position_token_id, block_number),
            self.position_liquidity(position_token_id, block_number),
        )?;

        let pool_id = pool_key.into();
        let slot0 = self.slot0_by_pool_id(pool_id, block_number).await?;

        Ok(position_fees(
            self.root(),
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            *ANGSTROM_ADDRESS.get().unwrap(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            block_number,
            pool_id,
            slot0.tick,
            position_token_id,
            position_info.tick_lower,
            position_info.tick_upper,
            position_liquidity
        )
        .await?)
    }
}

#[cfg(test)]
mod data_api_tests {

    use alloy_primitives::aliases::U24;

    use super::*;
    use crate::l1::{
        test_utils::{USDC, WETH,
valid_test_params::init_valid_position_params_with_provider},
        types::{HistoricalOrdersFilter, OrderKind}
    };

    #[tokio::test]
    async fn test_fetch_fee_configuration() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let fee_config = provider
            .fee_configuration_by_tokens(USDC, WETH, None,
Some(state.block_number))             .await
            .unwrap();

        assert_eq!(
            FeeConfiguration { bundle_fee: 200, swap_fee: 238, protocol_fee:
112 },             fee_config
        );
    }

    #[tokio::test]
    async fn test_tokens_by_partial_pool_key() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let token_pair = provider
            .tokens_by_partial_pool_key(
                AngstromPoolConfigStore::derive_store_key(
                    state.pool_key.currency0,
                    state.pool_key.currency1
                ),
                Some(state.block_number)
            )
            .await
            .unwrap();

        assert_eq!(token_pair.token0, state.pool_key.currency0);
        assert_eq!(token_pair.token1, state.pool_key.currency1);
    }

    #[tokio::test]
    async fn test_all_token_pairs_with_config_store() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let config_store = provider
            .pool_config_store(Some(state.block_number))
            .await
            .unwrap();

        let all_pairs = provider
            .all_token_pairs_with_config_store(config_store,
Some(state.block_number))             .await
            .unwrap();

        assert_eq!(all_pairs.len(), 2);
        assert!(!all_pairs.contains(&TokenPair { token0: Address::ZERO,
token1: Address::ZERO }));     }

    #[tokio::test]
    async fn test_all_token_pairs() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let all_pairs = provider
            .all_token_pairs(Some(state.block_number))
            .await
            .unwrap();

        assert_eq!(all_pairs.len(), 2);
        assert!(!all_pairs.contains(&TokenPair { token0: Address::ZERO,
token1: Address::ZERO }));     }

    #[tokio::test]
    async fn test_all_tokens() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let all_tokens =
provider.all_tokens(Some(state.block_number)).await.unwrap();

        assert_eq!(all_tokens.len(), 3);
        assert!(!all_tokens.contains(&Address::ZERO));
    }

    #[tokio::test]
    async fn test_all_tokens_with_config_store() {
        let (provider, state) =
init_valid_position_params_with_provider().await;         let config_store =
provider             .pool_config_store(Some(state.block_number))
            .await
            .unwrap();

        let all_tokens = provider
            .all_tokens_with_config_store(config_store,
Some(state.block_number))             .await
            .unwrap();

        assert_eq!(all_tokens.len(), 3);
        assert!(!all_tokens.contains(&Address::ZERO));
    }

    #[tokio::test]
    async fn test_pool_key_by_tokens() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let pool_key = provider
            .pool_key_by_tokens(
                state.pool_key.currency0,
                state.pool_key.currency1,
                Some(state.block_number)
            )
            .await
            .unwrap();

        assert_eq!(
            pool_key,
            PoolKeyWithAngstromFee {
                pool_fee_in_e6: U24::from(200_u16),
                pool_key:       state.pool_key
            }
        );
    }

    #[tokio::test]
    async fn test_pool_key_by_pool_id() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let pool_key = provider
            .pool_key_by_pool_id(state.pool_key.into(),
Some(state.block_number))             .await
            .unwrap();

        assert_eq!(
            pool_key,
            PoolKeyWithAngstromFee {
                pool_fee_in_e6: U24::from(200_u16),
                pool_key:       state.pool_key
            }
        );
    }

    #[tokio::test]
    async fn test_pool_id() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let pool_key = provider
            .pool_id(state.pool_key.currency0, state.pool_key.currency1,
Some(state.block_number))             .await
            .unwrap();

        assert_eq!(pool_key, PoolId::from(state.pool_key));
    }

    #[tokio::test]
    async fn test_pool_key_by_pool_id_with_config_store() {
        let (provider, state) =
init_valid_position_params_with_provider().await;         let config_store =
provider             .pool_config_store(Some(state.block_number))
            .await
            .unwrap();

        let pool_key = provider
            .pool_key_by_pool_id_with_config_store(
                state.pool_key.into(),
                config_store,
                Some(state.block_number)
            )
            .await
            .unwrap();

        assert_eq!(
            pool_key,
            PoolKeyWithAngstromFee {
                pool_fee_in_e6: U24::from(200_u16),
                pool_key:       state.pool_key
            }
        );
    }

    #[tokio::test]
    async fn test_historical_orders() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let filter = HistoricalOrdersFilter::new()
            .from_block(state.valid_block_after_swaps)
            .to_block(state.valid_block_after_swaps)
            .order_kind(OrderKind::User);
        let orders = provider.historical_orders(filter, None).await.unwrap();

        assert_eq!(orders.len(), 1);
    }

    #[tokio::test]
    async fn test_historical_bundles() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let orders = provider
            .historical_bundles(
                Some(state.valid_block_after_swaps),
                Some(state.valid_block_after_swaps),
                None
            )
            .await
            .unwrap();

        assert_eq!(orders.len(), 1);
    }

    #[tokio::test]
    async fn test_historical_liquidity_changes() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let modify_liquidity = provider
            .historical_liquidity_changes(
                Some(state.block_for_liquidity_add),
                Some(state.block_for_liquidity_add)
            )
            .await
            .unwrap();

        assert_eq!(modify_liquidity.len(), 1);
    }

    #[tokio::test]
    async fn test_pool_data_by_tokens() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let (_, pool_data) = provider
            .pool_data_by_tokens(
                state.pool_key.currency0,
                state.pool_key.currency1,
                true,
                Some(state.block_number)
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
    async fn test_pool_data_by_pool_id() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let (_, pool_data) = provider
            .pool_data_by_pool_id(PoolId::from(state.pool_key), true,
Some(state.block_number))             .await
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
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let all_pool_data = provider
            .all_pool_data(true, Some(state.block_number))
            .await
            .unwrap();

        assert_eq!(all_pool_data.len(), 2);
    }

    #[tokio::test]
    async fn test_pool_config_store() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let config_store = provider
            .pool_config_store(Some(state.block_number))
            .await
            .unwrap();

        assert_eq!(config_store.all_entries().len(), 2);
    }

    #[tokio::test]
    async fn test_slot0_by_pool_id() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let slot0 = provider
            .slot0_by_pool_id(PoolId::from(state.pool_key),
Some(state.block_number))             .await
            .unwrap();

        assert_eq!(slot0.tick, state.current_pool_tick);
    }

    #[tokio::test]
    async fn test_slot0_by_tokens() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let slot0 = provider
            .slot0_by_tokens(
                state.pool_key.currency0,
                state.pool_key.currency1,
                Some(state.block_number)
            )
            .await
            .unwrap();

        assert_eq!(slot0.tick, state.current_pool_tick);
    }

    #[tokio::test]
    async fn test_get_bundle_by_block() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let bundle = provider
            .get_bundle_by_block(state.valid_block_after_swaps, true)
            .await
            .unwrap();

        assert!(bundle.is_some());
    }

    #[tokio::test]
    async fn test_get_bundle_by_tx_hash() {
        let (provider, state) =
init_valid_position_params_with_provider().await;

        let bundle = provider
            .get_bundle_by_tx_hash(state.bundle_tx_hash, true)
            .await
            .unwrap();

        assert!(bundle.is_some());
    }
}

#[cfg(test)]
mod user_api_tests {

    use alloy_primitives::U256;

    use crate::{
        l1::test_utils::valid_test_params::init_valid_position_params_with_provider,
        types::fees::LiquidityPositionFees
    };

    #[tokio::test]
    async fn test_position_and_pool_info_by_token_id() {
        let (provider, pos_info) =
init_valid_position_params_with_provider().await;         let block_number =
pos_info.block_for_liquidity_add + 1;

        let (pool_key, unpacked_position_info) = provider
            .position_and_pool_info(pos_info.position_token_id,
Some(block_number))             .await
            .unwrap();

        assert_eq!(pool_key, pos_info.pool_key);
        assert_eq!(unpacked_position_info,
pos_info.as_unpacked_position_info());     }

    #[tokio::test]
    async fn test_position_liquidity_by_token_id() {
        let (provider, pos_info) =
init_valid_position_params_with_provider().await;         let block_number =
pos_info.block_for_liquidity_add + 1;

        let position_liquidity = provider
            .position_liquidity(pos_info.position_token_id,
Some(block_number))             .await
            .unwrap();

        assert_eq!(pos_info.position_liquidity, position_liquidity);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_all_user_positions() {
        let (provider, pos_info) =
init_valid_position_params_with_provider().await;         let block_number =
pos_info.block_for_liquidity_add + 1;

        let bound: u64 = 10;

        let position_liquidity = provider
            .all_user_positions(
                pos_info.owner,
                pos_info.position_token_id - U256::from(bound),
                pos_info.position_token_id + U256::from(bound),
                None,
                None,
                Some(block_number)
            )
            .await
            .unwrap();

        assert_eq!(position_liquidity.len(), 1);
    }

    #[tokio::test]
    async fn test_user_position_fees() {
        let (provider, pos_info) =
init_valid_position_params_with_provider().await;         let block_number =
pos_info.block_for_liquidity_add + 100;

        let results = provider
            .user_position_fees(pos_info.position_token_id,
Some(block_number))             .await
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
*/
