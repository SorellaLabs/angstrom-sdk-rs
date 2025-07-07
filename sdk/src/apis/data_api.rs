use std::{
    collections::{HashMap, HashSet},
    sync::Arc
};

use alloy_consensus::Transaction;
use alloy_eips::BlockId;
use alloy_primitives::{
    Address, FixedBytes, TxHash,
    aliases::{I24, U24}
};
use alloy_provider::Provider;
use alloy_sol_types::{SolCall, SolEvent};
use angstrom_types::{
    contract_bindings::{
        angstrom::Angstrom,
        controller_v_1::ControllerV1::getPoolByKeyCall,
        pool_manager::PoolManager::{self, PoolKey}
    },
    contract_payloads::angstrom::{
        AngstromBundle, AngstromPoolConfigStore, AngstromPoolPartialKey
    },
    primitive::{ANGSTROM_ADDRESS, CONTROLLER_V1_ADDRESS, POOL_MANAGER_ADDRESS, PoolId}
};
use futures::{FutureExt, StreamExt};
use itertools::Itertools;
use pade::PadeDecode;
use uniswap_v4::uniswap::{
    pool::EnhancedUniswapPool, pool_data_loader::DataLoader, pool_factory::INITIAL_TICKS_PER_SIDE
};

use super::utils::*;
use crate::types::{
    positions::{pool_manager_pool_slot0, utils::UnpackedSlot0},
    *
};

#[async_trait::async_trait]
pub trait AngstromDataApi: Send + Sized {
    async fn all_token_pairs(&self, block_number: Option<u64>) -> eyre::Result<Vec<TokenPair>> {
        let config_store = self.pool_config_store(block_number).await?;
        self.all_token_pairs_with_config_store(config_store, block_number)
            .await
    }

    async fn all_token_pairs_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<TokenPair>>;

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
    ) -> eyre::Result<Option<PoolKeyWithAngstromFee>> {
        let config_store = self.pool_config_store(block_number).await?;
        self.pool_key_by_pool_id_with_config_store(pool_id, config_store, block_number)
            .await
    }

    async fn pool_key_by_pool_id_with_config_store(
        &self,
        pool_id: PoolId,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>
    ) -> eyre::Result<Option<PoolKeyWithAngstromFee>> {
        Ok(self
            .all_pool_keys_with_config_store(config_store, block_number)
            .await?
            .into_iter()
            .find(|pool_key| pool_id == PoolId::from(pool_key)))
    }

    async fn tokens_by_partial_pool_key(
        &self,
        partial_pool_key: AngstromPoolPartialKey,
        block_number: Option<u64>
    ) -> eyre::Result<TokenPair>;

    async fn all_pool_keys(
        &self,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<PoolKeyWithAngstromFee>> {
        let config_store = self.pool_config_store(block_number).await?;
        self.all_pool_keys_with_config_store(config_store, block_number)
            .await
    }

    async fn all_pool_keys_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<PoolKeyWithAngstromFee>> {
        let all_token_pairs = self
            .all_token_pairs_with_config_store(config_store.clone(), block_number)
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
                    hooks:       *ANGSTROM_ADDRESS.get().unwrap()
                };
                PoolKeyWithAngstromFee { pool_key, pool_fee_in_e6: U24::from(v.fee_in_e6) }
            })
            .collect())
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

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<WithEthMeta<Vec<HistoricalOrders>>>> {
        let bundles = self
            .historical_bundles(filter.from_block, filter.to_block, block_stream_buffer)
            .await?;

        if bundles.is_empty() {
            return Ok(Vec::new())
        }

        let pool_stores = AngstromPoolTokenIndexToPair::new_with_tokens(self, &filter).await?;

        Ok(bundles
            .into_iter()
            .map(|bundle| bundle.map_inner(|b| filter.filter_bundle(b, &pool_stores)))
            .collect())
    }

    async fn historical_bundles(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<WithEthMeta<AngstromBundle>>>;

    async fn historical_liquidity_changes(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>>;

    async fn historical_post_bundle_unlock_swaps(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::Swap>>>;

    async fn get_bundle_by_block(
        &self,
        block_number: u64,
        verify_successful_tx: bool
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
        block_number: Option<u64>
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)>;

    async fn pool_data_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        let pool_key = self
            .pool_key_by_pool_id(pool_id, block_number)
            .await?
            .ok_or_else(|| eyre::eyre!("pool key does not exist for these tokens"))?;
        self.pool_data_by_tokens(
            pool_key.pool_key.currency0,
            pool_key.pool_key.currency1,
            block_number
        )
        .await
    }

    async fn all_pool_data(
        &self,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<(u64, EnhancedUniswapPool<DataLoader>)>> {
        let token_pairs = self.all_token_pairs(block_number).await?;

        let pools = futures::future::try_join_all(
            token_pairs
                .into_iter()
                .map(|pair| self.pool_data_by_tokens(pair.token0, pair.token1, block_number))
        )
        .await?;

        Ok(pools)
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>
    ) -> eyre::Result<AngstromPoolConfigStore>;

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
}

#[async_trait::async_trait]
impl<P: Provider> AngstromDataApi for P {
    async fn tokens_by_partial_pool_key(
        &self,
        pool_partial_key: AngstromPoolPartialKey,
        block_number: Option<u64>
    ) -> eyre::Result<TokenPair> {
        let out = view_call(
            self,
            block_number,
            *CONTROLLER_V1_ADDRESS.get().unwrap(),
            getPoolByKeyCall { key: FixedBytes::from(*pool_partial_key) }
        )
        .await??;

        Ok(TokenPair { token0: out.asset0, token1: out.asset1 })
    }

    async fn all_token_pairs_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<TokenPair>> {
        let partial_key_entries = config_store.all_entries();
        let token_pairs = futures::future::try_join_all(
            partial_key_entries
                .iter()
                .map(|key| self.tokens_by_partial_pool_key(key.pool_partial_key, block_number))
        )
        .await?;

        Ok(token_pairs)
    }

    async fn pool_key_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<PoolKeyWithAngstromFee> {
        let (token0, token1) = sort_tokens(token0, token1);

        let config_store = self.pool_config_store(block_number).await?;
        let pool_config_store = config_store
            .get_entry(token0, token1)
            .ok_or(eyre::eyre!("no config store entry for tokens {token0:?} - {token1:?}"))?;

        let pool_key = PoolKey {
            currency0:   token0,
            currency1:   token1,
            fee:         U24::from(0x800000),
            tickSpacing: I24::unchecked_from(pool_config_store.tick_spacing),
            hooks:       *ANGSTROM_ADDRESS.get().unwrap()
        };

        Ok(PoolKeyWithAngstromFee {
            pool_key,
            pool_fee_in_e6: U24::from(pool_config_store.fee_in_e6)
        })
    }

    async fn historical_bundles(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<WithEthMeta<AngstromBundle>>> {
        let filter = historical_pool_manager_swap_filter(start_block, end_block);
        let logs = self.get_logs(&filter).await?;

        let blocks_with_bundles = logs.into_iter().flat_map(|log| {
            let swap_log = PoolManager::Swap::decode_log(&log.inner).ok()?;
            (swap_log.fee == U24::ZERO)
                .then(|| log.block_number)
                .flatten()
        });

        let mut bundle_stream = futures::stream::iter(blocks_with_bundles)
            .map(|block_number| self.get_bundle_by_block(block_number, true))
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
        end_block: Option<u64>
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>> {
        let all_pool_ids = self
            .all_pool_keys(end_block)
            .await?
            .into_iter()
            .map(|val| PoolId::from(val.pool_key))
            .collect::<HashSet<_>>();

        let filter = historical_pool_manager_modify_liquidity_filter(start_block, end_block);
        let logs = self.get_logs(&filter).await?;

        Ok(logs
            .into_iter()
            .flat_map(|log| {
                PoolManager::ModifyLiquidity::decode_log(&log.inner)
                    .ok()
                    .map(|inner_log| {
                        all_pool_ids.contains(&inner_log.id).then(|| {
                            WithEthMeta::new(
                                log.block_number,
                                log.transaction_hash,
                                log.transaction_index,
                                inner_log.data
                            )
                        })
                    })
                    .flatten()
            })
            .collect())
    }

    async fn historical_post_bundle_unlock_swaps(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::Swap>>> {
        let all_pool_ids = self
            .all_pool_keys(end_block)
            .await?
            .into_iter()
            .map(|val| PoolId::from(val.pool_key))
            .collect::<HashSet<_>>();

        let filter = historical_pool_manager_swap_filter(start_block, end_block);
        let logs = self.get_logs(&filter).await?;

        Ok(logs
            .into_iter()
            .flat_map(|log| {
                PoolManager::Swap::decode_log(&log.inner)
                    .ok()
                    .map(|inner_log| {
                        all_pool_ids.contains(&inner_log.id).then(|| {
                            WithEthMeta::new(
                                log.block_number,
                                log.transaction_hash,
                                log.transaction_index,
                                inner_log.data
                            )
                        })
                    })
                    .flatten()
            })
            .collect())
    }

    async fn pool_data_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        let (token0, token1) = sort_tokens(token0, token1);

        let pool_key = self
            .pool_key_by_tokens(token0, token1, block_number)
            .await?;

        let public_pool_id = pool_key.clone().into();
        let private_pool_id: PoolId = pool_key.clone().into();
        let registry = vec![pool_key.as_angstrom_pool_key_type()].into();

        let data_loader = DataLoader::new_with_registry(
            private_pool_id,
            public_pool_id,
            registry,
            *POOL_MANAGER_ADDRESS.get().unwrap()
        );

        let mut enhanced_uni_pool = EnhancedUniswapPool::new(data_loader, INITIAL_TICKS_PER_SIDE);

        let block_number =
            if let Some(bn) = block_number { bn } else { self.get_block_number().await? };

        enhanced_uni_pool
            .initialize(Some(block_number), Arc::new(self))
            .boxed()
            .await?;

        Ok((block_number, enhanced_uni_pool))
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>
    ) -> eyre::Result<AngstromPoolConfigStore> {
        AngstromPoolConfigStore::load_from_chain(
            *ANGSTROM_ADDRESS.get().unwrap(),
            block_number.map(Into::into).unwrap_or(BlockId::latest()),
            self
        )
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))
    }

    async fn slot0_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>
    ) -> eyre::Result<UnpackedSlot0> {
        Ok(pool_manager_pool_slot0(
            self.root(),
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            block_number,
            pool_id
        )
        .await?)
    }

    async fn get_bundle_by_block(
        &self,
        block_number: u64,
        verify_successful_tx: bool
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>> {
        let Some(block) = self.get_block(block_number.into()).full().await? else {
            return Ok(None)
        };

        let angstrom_address = *ANGSTROM_ADDRESS.get().unwrap();

        let mut angstrom_bundles = block
            .transactions
            .into_transactions()
            .filter(|tx| tx.to() == Some(angstrom_address))
            .filter_map(|transaction| {
                let input: &[u8] = transaction.input();
                let call = Angstrom::executeCall::abi_decode(input).ok()?;
                let mut input = call.encoded.as_ref();
                Some((
                    *transaction.inner.tx_hash(),
                    WithEthMeta::new(
                        transaction.block_number,
                        Some(*transaction.inner.tx_hash()),
                        transaction.transaction_index,
                        AngstromBundle::pade_decode(&mut input, None).ok()?
                    )
                ))
            });

        if verify_successful_tx {
            let bundles =
                futures::future::try_join_all(angstrom_bundles.map(async |(tx_hash, bundle)| {
                    if self
                        .get_transaction_receipt(tx_hash)
                        .await?
                        .unwrap()
                        .status()
                    {
                        Ok::<_, eyre::ErrReport>(Some(bundle))
                    } else {
                        Ok(None)
                    }
                }))
                .await?;
            Ok(bundles.into_iter().flatten().next())
        } else {
            Ok(angstrom_bundles.next().map(|(_, bundle)| bundle))
        }
    }

    async fn get_bundle_by_tx_hash(
        &self,
        tx_hash: TxHash,
        verify_successful_tx: bool
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>> {
        let Some(transaction) = self.get_transaction_by_hash(tx_hash).await? else {
            return Ok(None)
        };

        if verify_successful_tx {
            if !self
                .get_transaction_receipt(tx_hash)
                .await?
                .ok_or_else(|| eyre::eyre!("reciepts not enabled on node - tx hash: {tx_hash:?}"))?
                .status()
            {
                return Ok(None);
            }
        }

        let input: &[u8] = transaction.input();
        Ok(Angstrom::executeCall::abi_decode(input)
            .ok()
            .map(|decoded| {
                let mut input = decoded.encoded.as_ref();
                Some(WithEthMeta::new(
                    transaction.block_number,
                    Some(*transaction.inner.tx_hash()),
                    transaction.transaction_index,
                    AngstromBundle::pade_decode(&mut input, None).ok()?
                ))
            })
            .flatten())
    }
}

#[cfg(test)]
mod tests {

    use alloy_primitives::aliases::U24;

    use super::*;
    use crate::test_utils::valid_test_params::init_valid_position_params_with_provider;

    #[tokio::test]
    async fn test_tokens_by_partial_pool_key() {
        let (provider, state) = init_valid_position_params_with_provider().await;

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
        let (provider, state) = init_valid_position_params_with_provider().await;

        let config_store = provider
            .pool_config_store(Some(state.block_number))
            .await
            .unwrap();

        let all_pairs = provider
            .all_token_pairs_with_config_store(config_store, Some(state.block_number))
            .await
            .unwrap();

        assert_eq!(all_pairs.len(), 3);
        assert!(!all_pairs.contains(&TokenPair { token0: Address::ZERO, token1: Address::ZERO }));
    }

    #[tokio::test]
    async fn test_all_token_pairs() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let all_pairs = provider
            .all_token_pairs(Some(state.block_number))
            .await
            .unwrap();

        assert_eq!(all_pairs.len(), 3);
        assert!(!all_pairs.contains(&TokenPair { token0: Address::ZERO, token1: Address::ZERO }));
    }

    #[tokio::test]
    async fn test_all_tokens() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let all_tokens = provider.all_tokens(Some(state.block_number)).await.unwrap();

        assert_eq!(all_tokens.len(), 3);
        assert!(!all_tokens.contains(&Address::ZERO));
    }

    #[tokio::test]
    async fn test_all_tokens_with_config_store() {
        let (provider, state) = init_valid_position_params_with_provider().await;
        let config_store = provider
            .pool_config_store(Some(state.block_number))
            .await
            .unwrap();

        let all_tokens = provider
            .all_tokens_with_config_store(config_store, Some(state.block_number))
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
                Some(state.block_number)
            )
            .await
            .unwrap();

        assert_eq!(
            pool_key,
            PoolKeyWithAngstromFee {
                pool_fee_in_e6: U24::from(350_u16),
                pool_key:       state.pool_key
            }
        );
    }

    #[tokio::test]
    async fn test_pool_key_by_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let pool_key = provider
            .pool_key_by_pool_id(state.pool_key.clone().into(), Some(state.block_number))
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            pool_key,
            PoolKeyWithAngstromFee {
                pool_fee_in_e6: U24::from(350_u16),
                pool_key:       state.pool_key
            }
        );
    }

    #[tokio::test]
    async fn test_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let pool_key = provider
            .pool_id(state.pool_key.currency0, state.pool_key.currency1, Some(state.block_number))
            .await
            .unwrap();

        assert_eq!(pool_key, PoolId::from(state.pool_key));
    }

    #[tokio::test]
    async fn test_pool_key_by_pool_id_with_config_store() {
        let (provider, state) = init_valid_position_params_with_provider().await;
        let config_store = provider
            .pool_config_store(Some(state.block_number))
            .await
            .unwrap();

        let pool_key = provider
            .pool_key_by_pool_id_with_config_store(
                state.pool_key.clone().into(),
                config_store,
                Some(state.block_number)
            )
            .await
            .unwrap()
            .unwrap();

        assert_eq!(
            pool_key,
            PoolKeyWithAngstromFee {
                pool_fee_in_e6: U24::from(350_u16),
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
        let orders = provider.historical_orders(filter, None).await.unwrap();

        assert_eq!(orders.len(), 1);
    }

    #[tokio::test]
    async fn test_historical_bundles() {
        let (provider, state) = init_valid_position_params_with_provider().await;

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
        let (provider, state) = init_valid_position_params_with_provider().await;

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
        let (provider, state) = init_valid_position_params_with_provider().await;

        let (_, pool_data) = provider
            .pool_data_by_tokens(
                state.pool_key.currency0,
                state.pool_key.currency1,
                Some(state.block_number)
            )
            .await
            .unwrap();

        assert_eq!(pool_data.token0, state.pool_key.currency0);
        assert_eq!(pool_data.token1, state.pool_key.currency1);
        assert_eq!(pool_data.private_address(), PoolId::from(state.pool_key));
    }

    #[tokio::test]
    async fn test_pool_data_by_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let (_, pool_data) = provider
            .pool_data_by_pool_id(PoolId::from(state.pool_key.clone()), Some(state.block_number))
            .await
            .unwrap();

        assert_eq!(pool_data.token0, state.pool_key.currency0);
        assert_eq!(pool_data.token1, state.pool_key.currency1);
        assert_eq!(pool_data.private_address(), PoolId::from(state.pool_key));
    }

    #[tokio::test]
    async fn test_all_pool_data() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let all_pool_data = provider
            .all_pool_data(Some(state.block_number))
            .await
            .unwrap();

        assert_eq!(all_pool_data.len(), 3);
    }

    #[tokio::test]
    async fn test_pool_config_store() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let config_store = provider
            .pool_config_store(Some(state.block_number))
            .await
            .unwrap();

        assert_eq!(config_store.all_entries().len(), 3);
    }

    #[tokio::test]
    async fn test_slot0_by_pool_id() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let slot0 = provider
            .slot0_by_pool_id(PoolId::from(state.pool_key.clone()), Some(state.block_number))
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
                Some(state.block_number)
            )
            .await
            .unwrap();

        assert_eq!(slot0.tick, state.current_pool_tick);
    }

    #[tokio::test]
    async fn test_get_bundle_by_block() {
        let (provider, state) = init_valid_position_params_with_provider().await;

        let bundle = provider
            .get_bundle_by_block(state.valid_block_after_swaps, true)
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
