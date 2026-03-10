use std::{
    collections::{HashMap, HashSet},
    ops::Deref
};

use alloy_consensus::Transaction;
use alloy_eips::BlockId;
use alloy_network::{Ethereum, TransactionResponse};
use alloy_primitives::{
    Address, FixedBytes, TxHash, U256,
    aliases::{I24, U24},
    keccak256
};
use alloy_sol_types::{SolCall, SolEvent};
use angstrom_types_primitives::{
    contract_bindings::{
        angstrom::Angstrom,
        controller_v_1::ControllerV1,
        pool_manager::PoolManager::{self, PoolKey}
    },
    contract_payloads::angstrom::{
        AngstromBundle, AngstromPoolConfigStore, AngstromPoolPartialKey
    },
    primitive::PoolId
};
use futures::StreamExt;
use itertools::Itertools;
use pade::PadeDecode;
use uni_v4::{
    BaselinePoolState, L1FeeConfiguration, PoolKey as UniPoolKey,
    baseline_pool_factory::INITIAL_TICKS_PER_SIDE,
    bindings::get_uniswap_v_4_pool_data::GetUniswapV4PoolData,
    liquidity_base::BaselineLiquidity,
    pool_data_loader::{PoolData, PoolDataV4}
};
use uniswap_storage::{
    StorageSlotFetcher,
    v4::{UnpackedSlot0, pool_manager::pool_state::pool_manager_pool_slot0}
};

use crate::{
    l1::{AngstromL1Chain, types::*},
    types::{
        common::*,
        pool_tick_loaders::{DEFAULT_TICKS_PER_BATCH, FullTickLoader, PoolTickDataLoader},
        utils::{
            historical_pool_manager_modify_liquidity_filter, historical_pool_manager_swap_filter
        }
    }
};

impl<P> AngstromL1DataApi for P where
    P: PoolTickDataLoader<Ethereum> + StorageSlotFetcher + Send + Sized
{
}

#[async_trait::async_trait]
pub trait AngstromL1DataApi:
    PoolTickDataLoader<Ethereum> + StorageSlotFetcher + Send + Sized
{
    async fn all_token_pairs(
        &self,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<TokenPair>> {
        let config_store = self.pool_config_store(block_id, chain).await?;
        self.all_token_pairs_with_config_store(config_store, block_id, chain)
            .await
    }

    async fn all_token_pairs_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<TokenPair>> {
        let partial_key_entries = config_store.all_entries();
        let token_pairs = futures::future::try_join_all(
            partial_key_entries
                .iter()
                .map(|key| self.tokens_by_partial_pool_key(key.pool_partial_key, block_id, chain))
        )
        .await?;

        Ok(token_pairs)
    }

    async fn all_tokens(
        &self,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<Address>> {
        let config_store = self.pool_config_store(block_id, chain).await?;
        self.all_tokens_with_config_store(config_store, block_id, chain)
            .await
    }

    async fn all_tokens_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<Address>> {
        Ok(self
            .all_token_pairs_with_config_store(config_store, block_id, chain)
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
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<PoolKeyWithAngstromFee> {
        let (token0, token1) = sort_tokens(token0, token1);

        let config_store = self.pool_config_store(block_id, chain).await?;
        let pool_config_store = config_store
            .get_entry(token0, token1)
            .ok_or(eyre::eyre!("no config store entry for tokens {token0:?} - {token1:?}"))?;

        let pool_key = PoolKey {
            currency0:   token0,
            currency1:   token1,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(pool_config_store.tick_spacing),
            hooks:       chain.constants().angstrom_address()
        };

        Ok(PoolKeyWithAngstromFee {
            pool_key,
            pool_fee_in_e6: U24::from(pool_config_store.fee_in_e6)
        })
    }

    async fn pool_key_by_pool_id(
        &self,
        pool_id: PoolId,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<PoolKeyWithAngstromFee> {
        let config_store = self.pool_config_store(block_id, chain).await?;
        self.pool_key_by_pool_id_with_config_store(pool_id, config_store, block_id, chain)
            .await
    }

    async fn pool_key_by_pool_id_with_config_store(
        &self,
        pool_id: PoolId,
        config_store: AngstromPoolConfigStore,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<PoolKeyWithAngstromFee> {
        self.all_pool_keys_with_config_store(config_store, block_id, chain)
            .await?
            .into_iter()
            .find(|pool_key| pool_id == PoolId::from(pool_key))
            .ok_or_else(|| eyre::eyre!("no pool key for pool_id: {pool_id:?}"))
    }

    async fn tokens_by_partial_pool_key(
        &self,
        partial_pool_key: AngstromPoolPartialKey,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<TokenPair> {
        let out = self
            .view_call(
                block_id,
                chain.constants().controller_v1_address(),
                ControllerV1::getPoolByKeyCall { key: FixedBytes::from(*partial_pool_key) }
            )
            .await?;

        Ok(TokenPair { token0: out.asset0, token1: out.asset1 })
    }

    async fn all_pool_keys(
        &self,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<PoolKeyWithAngstromFee>> {
        let config_store = self.pool_config_store(block_id, chain).await?;
        self.all_pool_keys_with_config_store(config_store, block_id, chain)
            .await
    }

    async fn all_pool_keys_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<PoolKeyWithAngstromFee>> {
        let all_token_pairs = self
            .all_token_pairs_with_config_store(config_store.clone(), block_id, chain)
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
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<PoolId> {
        self.pool_key_by_tokens(token0, token1, block_id, chain)
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
            return Ok(Vec::new());
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
    ) -> eyre::Result<Vec<WithEthMeta<AngstromBundle>>> {
        let consts = chain.constants();
        let filters = historical_pool_manager_swap_filter(
            start_block,
            end_block,
            consts.uniswap_constants().pool_manager(),
            consts.angstrom_deploy_block()
        );
        let logs = futures::future::try_join_all(
            filters
                .into_iter()
                .map(async |filter| self.fetch_logs(&filter).await)
        )
        .await?;

        let blocks_with_bundles = logs.into_iter().flatten().flat_map(|log| {
            let swap_log = PoolManager::Swap::decode_log(&log.inner).ok()?;
            (swap_log.fee == U24::ZERO)
                .then_some(log.block_number)
                .flatten()
        });

        let mut bundle_stream = futures::stream::iter(blocks_with_bundles)
            .map(|block_id| self.get_bundle_by_block(block_id.into(), true, chain))
            .buffer_unordered(block_stream_buffer.unwrap_or(100));

        let mut all_bundles = Vec::new();
        while let Some(val) = bundle_stream.next().await {
            all_bundles.extend(val?);
        }

        Ok(all_bundles)
    }

    async fn historical_liquidity_changes(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>> {
        let all_pool_ids = self
            .all_pool_keys(end_block.map(Into::into).unwrap_or_else(BlockId::latest), chain)
            .await?
            .into_iter()
            .map(|val| PoolId::from(val.pool_key))
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
                .map(async |filter| self.fetch_logs(&filter).await)
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

    async fn historical_post_bundle_unlock_swaps(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::Swap>>> {
        let all_pool_ids = self
            .all_pool_keys(end_block.map(Into::into).unwrap_or_else(BlockId::latest), chain)
            .await?
            .into_iter()
            .map(|val| PoolId::from(val.pool_key))
            .collect::<HashSet<_>>();

        let consts = chain.constants();
        let filters = historical_pool_manager_swap_filter(
            start_block,
            end_block,
            consts.uniswap_constants().pool_manager(),
            consts.angstrom_deploy_block()
        );
        let logs = futures::future::try_join_all(
            filters
                .into_iter()
                .map(async |filter| self.fetch_logs(&filter).await)
        )
        .await?;

        Ok(logs
            .into_iter()
            .flatten()
            .flat_map(|log| {
                PoolManager::Swap::decode_log(&log.inner)
                    .ok()
                    .and_then(|inner_log| {
                        (all_pool_ids.contains(&inner_log.id) && !inner_log.fee.is_zero()).then(
                            || {
                                WithEthMeta::new(
                                    log.block_number,
                                    log.transaction_hash,
                                    log.transaction_index,
                                    inner_log.data
                                )
                            }
                        )
                    })
            })
            .collect())
    }

    async fn get_bundle_by_block(
        &self,
        block_id: BlockId,
        verify_successful_tx: bool,
        chain: AngstromL1Chain
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>> {
        let block = self.fetch_block(block_id, true).await?;
        let angstrom_address = chain.constants().angstrom_address();

        let txs = block
            .transactions
            .into_transactions()
            .filter(|tx| tx.to() == Some(angstrom_address));

        let angstrom_bundles = futures::stream::iter(txs)
            .filter_map(|transaction| async move {
                if verify_successful_tx
                    && !self
                        .tx_success(transaction.tx_hash())
                        .await
                        .unwrap_or_default()
                {
                    return None;
                }

                let input: &[u8] = transaction.input();
                let call = Angstrom::executeCall::abi_decode(input).ok()?;
                let mut input = call.encoded.as_ref();

                let tx_idx = transaction.transaction_index?;
                let decoded_input = AngstromBundle::pade_decode(&mut input, None).ok()?;

                Some((
                    tx_idx,
                    WithEthMeta::new(
                        transaction.block_number(),
                        Some(transaction.tx_hash()),
                        transaction.transaction_index(),
                        decoded_input
                    )
                ))
            })
            .collect::<Vec<_>>()
            .await;

        Ok(angstrom_bundles
            .into_iter()
            .sorted_by_key(|(idx, _)| *idx)
            .map(|(_, bundle)| bundle)
            .next())
    }

    async fn get_bundle_by_tx_hash(
        &self,
        tx_hash: TxHash,
        verify_successful_tx: bool
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>> {
        let Some(transaction) = self.tx_by_hash(tx_hash).await? else {
            return Ok(None);
        };

        if verify_successful_tx && !self.tx_success(tx_hash).await? {
            return Ok(None);
        }

        let input: &[u8] = transaction.input();
        Ok(Angstrom::executeCall::abi_decode(input)
            .ok()
            .and_then(|decoded| {
                let mut input = decoded.encoded.as_ref();
                Some(WithEthMeta::new(
                    transaction.block_number(),
                    Some(transaction.tx_hash()),
                    transaction.transaction_index(),
                    AngstromBundle::pade_decode(&mut input, None).ok()?
                ))
            }))
    }

    async fn pool_data_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        load_ticks: bool,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<(u64, BaselinePoolStateWithKey<Ethereum>)> {
        let (token0, token1) = sort_tokens(token0, token1);

        let pool_key = self
            .pool_key_by_tokens(token0, token1, block_id, chain)
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
            &self.alloy_root_provider().await?,
            pool_id,
            chain.constants().uniswap_constants().pool_manager(),
            pool_key.pool_key.currency0,
            pool_key.pool_key.currency1
        )
        .into_transaction_request();

        let out_pool_data = self
            .view_deploy_call::<PoolDataV4>(block_id, data_deployer_call)
            .await?;
        let pool_data: PoolData = (uni_pool_key, out_pool_data).into();

        let fee_config = self
            .fee_configuration_by_tokens(
                pool_key.pool_key.currency0,
                pool_key.pool_key.currency1,
                Some(pool_key.pool_fee_in_e6),
                block_id,
                chain
            )
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

        let baseline_liquidity = BaselineLiquidity::new(
            tick_spacing,
            tick,
            sqrt_price_x96,
            liquidity,
            ticks,
            tick_bitmap
        );

        let block_number = self.block_number_from_block_id(block_id).await?;

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

    async fn pool_data_by_pool_id(
        &self,
        pool_id: PoolId,
        load_ticks: bool,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<(u64, BaselinePoolStateWithKey<Ethereum>)> {
        let pool_key = self.pool_key_by_pool_id(pool_id, block_id, chain).await?;
        self.pool_data_by_tokens(
            pool_key.pool_key.currency0,
            pool_key.pool_key.currency1,
            load_ticks,
            block_id,
            chain
        )
        .await
    }

    async fn all_pool_data(
        &self,
        load_ticks: bool,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<Vec<(u64, BaselinePoolStateWithKey<Ethereum>)>> {
        let token_pairs = self.all_token_pairs(block_id, chain).await?;

        let pools = futures::future::try_join_all(token_pairs.into_iter().map(|pair| {
            self.pool_data_by_tokens(pair.token0, pair.token1, load_ticks, block_id, chain)
        }))
        .await?;

        Ok(pools)
    }

    async fn pool_config_store(
        &self,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<AngstromPoolConfigStore> {
        AngstromPoolConfigStore::load_from_chain(
            chain.constants().angstrom_address(),
            block_id,
            &self.alloy_root_provider().await?
        )
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))
    }

    async fn slot0_by_pool_id(
        &self,
        pool_id: PoolId,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<UnpackedSlot0> {
        Ok(pool_manager_pool_slot0(
            self,
            chain.constants().uniswap_constants().pool_manager(),
            pool_id,
            block_id
        )
        .await?)
    }

    async fn slot0_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<UnpackedSlot0> {
        let pool_id = self.pool_id(token0, token1, block_id, chain).await?;
        self.slot0_by_pool_id(pool_id, block_id, chain).await
    }

    async fn fee_configuration_by_pool_id(
        &self,
        pool_id: PoolId,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<L1FeeConfiguration> {
        let pool_key = self.pool_key_by_pool_id(pool_id, block_id, chain).await?;
        self.fee_configuration_by_tokens(
            pool_key.pool_key.currency0,
            pool_key.pool_key.currency1,
            Some(pool_key.pool_fee_in_e6),
            block_id,
            chain
        )
        .await
    }

    async fn fee_configuration_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        bundle_fee: Option<U24>,
        block_id: BlockId,
        chain: AngstromL1Chain
    ) -> eyre::Result<L1FeeConfiguration> {
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
            self.pool_key_by_tokens(token0, token1, block_id, chain)
                .await?
                .pool_fee_in_e6
        };

        let raw = self
            .view_call(
                block_id,
                chain.constants().angstrom_address(),
                Angstrom::extsloadCall { slot: U256::from_be_bytes(*slot) }
            )
            .await?;

        let bytes = raw.to_be_bytes::<32>();
        let unlocked_fee = U24::from_be_bytes([bytes[29], bytes[30], bytes[31]]);
        let protocol_fee = U24::from_be_bytes([bytes[26], bytes[27], bytes[28]]);

        Ok(L1FeeConfiguration {
            bundle_fee:   bundle_fee.to::<u32>(),
            swap_fee:     unlocked_fee.to::<u32>(),
            protocol_fee: protocol_fee.to::<u32>()
        })
    }
}

#[cfg(test)]
mod data_api_tests {

    use alloy_primitives::aliases::U24;

    use super::*;
    use crate::l1::{
        AngstromL1Chain,
        test_utils::{USDC, WETH, valid_test_params::init_valid_position_params_with_provider},
        types::{HistoricalOrdersFilter, OrderKind}
    };

    #[tokio::test]
    async fn test_fetch_fee_configuration() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let fee_config = provider
            .fee_configuration_by_tokens(
                USDC,
                WETH,
                None,
                state.block_number.into(),
                AngstromL1Chain::Mainnet
            )
            .await
            .unwrap();

        assert_eq!(
            L1FeeConfiguration { bundle_fee: 200, swap_fee: 238, protocol_fee: 112 },
            fee_config
        );
    }

    #[tokio::test]
    async fn test_tokens_by_partial_pool_key() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let token_pair = provider
            .tokens_by_partial_pool_key(
                AngstromPoolConfigStore::derive_store_key(
                    state.pool_key.currency0,
                    state.pool_key.currency1
                ),
                state.block_number.into(),
                AngstromL1Chain::Mainnet
            )
            .await
            .unwrap();

        assert_eq!(token_pair.token0, state.pool_key.currency0);
        assert_eq!(token_pair.token1, state.pool_key.currency1);
    }

    #[tokio::test]
    async fn test_all_token_pairs_with_config_store() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let config_store = provider
            .pool_config_store(state.block_number.into(), AngstromL1Chain::Mainnet)
            .await
            .unwrap();

        let all_pairs = provider
            .all_token_pairs_with_config_store(
                config_store,
                state.block_number.into(),
                AngstromL1Chain::Mainnet
            )
            .await
            .unwrap();

        assert_eq!(all_pairs.len(), 2);
        assert!(!all_pairs.contains(&TokenPair { token0: Address::ZERO, token1: Address::ZERO }));
    }

    #[tokio::test]
    async fn test_all_token_pairs() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let all_pairs = provider
            .all_token_pairs(state.block_number.into(), AngstromL1Chain::Mainnet)
            .await
            .unwrap();

        assert_eq!(all_pairs.len(), 2);
        assert!(!all_pairs.contains(&TokenPair { token0: Address::ZERO, token1: Address::ZERO }));
    }

    #[tokio::test]
    async fn test_all_tokens() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let all_tokens = provider
            .all_tokens(state.block_number.into(), AngstromL1Chain::Mainnet)
            .await
            .unwrap();

        assert_eq!(all_tokens.len(), 3);
        assert!(!all_tokens.contains(&Address::ZERO));
    }

    #[tokio::test]
    async fn test_all_tokens_with_config_store() {
        let (provider, state) = init_valid_position_params_with_provider().await;
        let config_store = provider
            .pool_config_store(state.block_number.into(), AngstromL1Chain::Mainnet)
            .await
            .unwrap();

        let all_tokens = provider
            .all_tokens_with_config_store(
                config_store,
                state.block_number.into(),
                AngstromL1Chain::Mainnet
            )
            .await
            .unwrap();

        assert_eq!(all_tokens.len(), 3);
        assert!(!all_tokens.contains(&Address::ZERO));
    }

    #[tokio::test]
    async fn test_pool_key_by_tokens() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let pool_key = provider
            .pool_key_by_tokens(
                state.pool_key.currency0,
                state.pool_key.currency1,
                state.block_number.into(),
                AngstromL1Chain::Mainnet
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
        let (provider, state) = init_valid_position_params_with_provider().await;

        let pool_key = provider
            .pool_key_by_pool_id(
                state.pool_key.into(),
                state.block_number.into(),
                AngstromL1Chain::Mainnet
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
    async fn test_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let pool_key = provider
            .pool_id(
                state.pool_key.currency0,
                state.pool_key.currency1,
                state.block_number.into(),
                AngstromL1Chain::Mainnet
            )
            .await
            .unwrap();

        assert_eq!(pool_key, PoolId::from(state.pool_key));
    }

    #[tokio::test]
    async fn test_pool_key_by_pool_id_with_config_store() {
        let (provider, state) = init_valid_position_params_with_provider().await;
        let config_store = provider
            .pool_config_store(state.block_number.into(), AngstromL1Chain::Mainnet)
            .await
            .unwrap();

        let pool_key = provider
            .pool_key_by_pool_id_with_config_store(
                state.pool_key.into(),
                config_store,
                state.block_number.into(),
                AngstromL1Chain::Mainnet
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
        let (provider, state) = init_valid_position_params_with_provider().await;

        let filter = HistoricalOrdersFilter::new()
            .from_block(state.valid_block_after_swaps)
            .to_block(state.valid_block_after_swaps)
            .order_kind(OrderKind::User);
        let orders = provider
            .historical_orders(filter, None, AngstromL1Chain::Mainnet)
            .await
            .unwrap();

        assert_eq!(orders.len(), 1);
    }

    #[tokio::test]
    async fn test_historical_bundles() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let orders = provider
            .historical_bundles(
                Some(state.valid_block_after_swaps),
                Some(state.valid_block_after_swaps),
                None,
                AngstromL1Chain::Mainnet
            )
            .await
            .unwrap();

        assert_eq!(orders.len(), 1);
    }

    #[tokio::test]
    async fn test_historical_liquidity_changes() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let modify_liquidity = provider
            .historical_liquidity_changes(
                Some(state.block_for_liquidity_add),
                Some(state.block_for_liquidity_add),
                AngstromL1Chain::Mainnet
            )
            .await
            .unwrap();

        assert_eq!(modify_liquidity.len(), 1);
    }

    #[tokio::test]
    async fn test_pool_data_by_tokens() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let (_, pool_data) = provider
            .pool_data_by_tokens(
                state.pool_key.currency0,
                state.pool_key.currency1,
                true,
                state.block_number.into(),
                AngstromL1Chain::Mainnet
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
        let (provider, state) = init_valid_position_params_with_provider().await;

        let (_, pool_data) = provider
            .pool_data_by_pool_id(
                PoolId::from(state.pool_key),
                true,
                state.block_number.into(),
                AngstromL1Chain::Mainnet
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
            .all_pool_data(true, state.block_number.into(), AngstromL1Chain::Mainnet)
            .await
            .unwrap();

        assert_eq!(all_pool_data.len(), 2);
    }

    #[tokio::test]
    async fn test_pool_config_store() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let config_store = provider
            .pool_config_store(state.block_number.into(), AngstromL1Chain::Mainnet)
            .await
            .unwrap();

        assert_eq!(config_store.all_entries().len(), 2);
    }

    #[tokio::test]
    async fn test_slot0_by_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let slot0 = provider
            .slot0_by_pool_id(
                PoolId::from(state.pool_key),
                state.block_number.into(),
                AngstromL1Chain::Mainnet
            )
            .await
            .unwrap();

        assert_eq!(slot0.tick, state.current_pool_tick);
    }

    #[tokio::test]
    async fn test_slot0_by_tokens() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let slot0 = provider
            .slot0_by_tokens(
                state.pool_key.currency0,
                state.pool_key.currency1,
                state.block_number.into(),
                AngstromL1Chain::Mainnet
            )
            .await
            .unwrap();

        assert_eq!(slot0.tick, state.current_pool_tick);
    }

    #[tokio::test]
    async fn test_get_bundle_by_block() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let bundle = provider
            .get_bundle_by_block(
                state.valid_block_after_swaps.into(),
                true,
                AngstromL1Chain::Mainnet
            )
            .await
            .unwrap();

        assert!(bundle.is_some());
    }

    #[tokio::test]
    async fn test_get_bundle_by_tx_hash() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let bundle = provider
            .get_bundle_by_tx_hash(state.bundle_tx_hash, true)
            .await
            .unwrap();

        assert!(bundle.is_some());
    }
}
