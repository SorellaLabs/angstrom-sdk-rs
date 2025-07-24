use std::{collections::HashSet, sync::Arc};

use alloy_consensus::Transaction;
use alloy_eips::BlockId;
use alloy_network::Ethereum;
use alloy_primitives::{
    Address, FixedBytes, TxHash, TxKind, U256,
    aliases::{I24, U24}
};
use alloy_provider::{Identity, Provider, ProviderBuilder, fillers::*};
use alloy_rpc_types::{Filter, Log};
use alloy_sol_types::{SolCall, SolEvent};
use angstrom_types::{
    contract_bindings::{
        angstrom::Angstrom,
        controller_v_1::ControllerV1,
        pool_manager::PoolManager::{self, PoolKey}
    },
    contract_payloads::angstrom::{
        AngstromBundle, AngstromPoolConfigStore, AngstromPoolPartialKey
    },
    primitive::{
        ANGSTROM_ADDRESS, CONTROLLER_V1_ADDRESS, POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS,
        PoolId
    },
    reth_db_provider::{RethDbLayer, RethDbProvider}
};
use futures::StreamExt;
use lib_reth::{EthApiServer, reth_libmdbx::RethLibmdbxClient, traits::EthRevm};
use pade::PadeDecode;
use reth_db::DatabaseEnv;
use reth_node_ethereum::EthereumNode;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_provider::providers::BlockchainProvider;
use revm::{ExecuteEvm, context::TxEnv};
use uniswap_v4::uniswap::{
    pool::EnhancedUniswapPool, pool_data_loader::DataLoader, pool_factory::INITIAL_TICKS_PER_SIDE
};

use crate::{
    apis::{
        AngstromDataApi, AngstromUserApi,
        utils::{
            historical_pool_manager_modify_liquidity_filter, historical_pool_manager_swap_filter
        }
    },
    types::{
        contracts::{
            UnpackedPositionInfo, UnpackedSlot0, UserLiquidityPosition,
            pool_manager::{
                pool_state::pool_manager_pool_slot0,
                position_state::pool_manager_position_state_liquidity
            },
            position_manager::{
                position_manager_next_token_id, position_manager_owner_of,
                position_manager_pool_key_and_info
            }
        },
        fees::{LiquidityPositionFees, position_fees},
        *
    }
};

pub type RethLayerProviderWrapperType<P> = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>
    >,
    RethDbProvider<P, BlockchainProvider<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>>
>;

#[derive(Clone)]
pub struct RethDbProviderWrapper<P: Provider + Clone> {
    db_client: Arc<RethLibmdbxClient>,
    provider:  P
}

impl<P: Provider + Clone> RethDbProviderWrapper<P> {
    pub fn new(db_client: Arc<RethLibmdbxClient>, provider: P) -> Self {
        Self { db_client, provider }
    }

    pub fn replace_provider(&mut self, provider: P) {
        self.provider = provider;
    }

    pub fn as_provider_with_db_layer(&self) -> RethLayerProviderWrapperType<P> {
        ProviderBuilder::<_, _, Ethereum>::default()
            .with_recommended_fillers()
            .layer(RethDbLayer::new(self.db_client.eth_db_provider().clone()))
            .connect_provider(self.provider.clone())
    }

    pub fn db_client(&self) -> Arc<RethLibmdbxClient> {
        self.db_client.clone()
    }

    async fn get_logs(&self, filter: &Filter) -> eyre::Result<Vec<Log>> {
        // let logs_res = self.db_client.eth_filter().logs(filter.clone()).await;
        // match logs_res {
        //     Ok(vals) => Ok(vals),
        //     Err(_) => {
        //         self.db_client()
        //             .eth_db_provider()
        //             .consistent_provider()?
        //             .static_file_provider()
        //             .initialize_index()?;
        //         tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        //         Ok(self.db_client.eth_filter().logs(filter.clone()).await?)
        //     }
        // }
        Ok(self.provider.get_logs(filter).await?)
    }
}

#[async_trait::async_trait]
impl<P: Provider + Clone> AngstromDataApi for RethDbProviderWrapper<P> {
    async fn tokens_by_partial_pool_key(
        &self,
        pool_partial_key: AngstromPoolPartialKey,
        block_number: Option<u64>
    ) -> eyre::Result<TokenPair> {
        let out = reth_db_view_call(
            &self.db_client,
            block_number,
            *CONTROLLER_V1_ADDRESS.get().unwrap(),
            ControllerV1::getPoolByKeyCall { key: FixedBytes::from(*pool_partial_key) }
        )??;

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
        let filters = historical_pool_manager_swap_filter(start_block, end_block);
        let logs = futures::future::try_join_all(
            filters
                .into_iter()
                .map(async move |filter| self.get_logs(&filter).await)
        )
        .await?;

        let blocks_with_bundles = logs.into_iter().flatten().flat_map(|log| {
            let swap_log = PoolManager::Swap::decode_log(&log.inner).ok()?;
            (swap_log.fee == U24::ZERO)
                .then_some(log.block_number)
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

        let filters = historical_pool_manager_swap_filter(start_block, end_block);
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

        let public_pool_id = pool_key.as_angstrom_pool_id();
        let private_pool_id: PoolId = pool_key.into();
        let registry = vec![pool_key.as_angstrom_pool_key_type()].into();

        let data_loader = DataLoader::new_with_registry(
            private_pool_id,
            public_pool_id,
            registry,
            *POOL_MANAGER_ADDRESS.get().unwrap()
        );

        let mut enhanced_uni_pool = EnhancedUniswapPool::new(data_loader, INITIAL_TICKS_PER_SIDE);

        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.db_client.eth_api().block_number()?.to()
        };

        enhanced_uni_pool
            .initialize(Some(block_number), Arc::new(self.as_provider_with_db_layer()))
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
            &self.as_provider_with_db_layer()
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
            self,
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
        let Some(block) = self
            .db_client
            .eth_api()
            .block_by_number(block_number.into(), true)
            .await?
        else {
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
                        .db_client
                        .eth_api()
                        .transaction_receipt(tx_hash)
                        .await?
                        .ok_or_else(|| {
                            eyre::eyre!("reciepts not enabled on node - tx hash: {tx_hash:?}")
                        })?
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
        let Some(transaction) = self
            .db_client
            .eth_api()
            .transaction_by_hash(tx_hash)
            .await?
        else {
            return Ok(None)
        };

        if verify_successful_tx
            && !self
                .db_client
                .eth_api()
                .transaction_receipt(tx_hash)
                .await?
                .ok_or_else(|| eyre::eyre!("reciepts not enabled on node - tx hash: {tx_hash:?}"))?
                .status()
        {
            return Ok(None);
        }

        let input: &[u8] = transaction.input();
        Ok(Angstrom::executeCall::abi_decode(input)
            .ok()
            .and_then(|decoded| {
                let mut input = decoded.encoded.as_ref();
                Some(WithEthMeta::new(
                    transaction.block_number,
                    Some(*transaction.inner.tx_hash()),
                    transaction.transaction_index,
                    AngstromBundle::pade_decode(&mut input, None).ok()?
                ))
            }))
    }
}

#[async_trait::async_trait]
impl<P: Provider + Clone> AngstromUserApi for RethDbProviderWrapper<P> {
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)> {
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            self,
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            block_number,
            position_token_id
        )
        .await?;

        Ok((pool_key, position_info))
    }

    async fn position_liquidity(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<u128> {
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            self,
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            block_number,
            position_token_id
        )
        .await?;

        let liquidity = pool_manager_position_state_liquidity(
            self,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            block_number,
            pool_key.into(),
            position_token_id,
            position_info.tick_lower,
            position_info.tick_upper
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
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        let position_manager_address = *POSITION_MANAGER_ADDRESS.get().unwrap();
        let pool_manager_address = *POOL_MANAGER_ADDRESS.get().unwrap();
        let angstrom_address = *ANGSTROM_ADDRESS.get().unwrap();

        if start_token_id == U256::ZERO {
            start_token_id = U256::from(1u8);
        }

        if end_token_id == U256::ZERO {
            end_token_id =
                position_manager_next_token_id(self, position_manager_address, block_number)
                    .await?;
        }

        let mut all_positions = Vec::new();
        while start_token_id <= end_token_id {
            let owner_of = position_manager_owner_of(
                self,
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
                self,
                position_manager_address,
                block_number,
                start_token_id
            )
            .await?;

            if pool_key.hooks != angstrom_address
                || pool_id
                    .map(|id| id != PoolId::from(pool_key))
                    .unwrap_or_default()
            {
                start_token_id += U256::from(1u8);
                continue;
            }

            let liquidity = pool_manager_position_state_liquidity(
                self,
                pool_manager_address,
                block_number,
                pool_key.into(),
                start_token_id,
                position_info.tick_lower,
                position_info.tick_upper
            )
            .await?;

            all_positions.push(UserLiquidityPosition {
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
            self,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            *ANGSTROM_ADDRESS.get().unwrap(),
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

pub(crate) fn reth_db_view_call<IC>(
    provider: &RethLibmdbxClient,
    block_number: Option<u64>,
    contract: Address,
    call: IC
) -> eyre::Result<Result<IC::Return, alloy_sol_types::Error>>
where
    IC: SolCall + Send
{
    let tx = TxEnv {
        kind: TxKind::Call(contract),
        data: call.abi_encode().into(),
        ..Default::default()
    };

    let block_number =
        if let Some(bn) = block_number { bn } else { provider.eth_api().block_number()?.to() };

    let mut evm = provider.make_empty_evm(block_number)?;

    let data = evm.transact(tx)?;

    Ok(IC::abi_decode_returns(data.result.output().unwrap_or_default()))
}
