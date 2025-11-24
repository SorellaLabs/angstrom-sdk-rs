use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::Arc
};

use alloy_consensus::Transaction;
use alloy_eips::BlockId;
use alloy_network::{Network, ReceiptResponse, TransactionResponse};
use alloy_primitives::{
    Address, FixedBytes, TxHash, TxKind, U256,
    aliases::{I24, U24},
    keccak256
};
use alloy_provider::{Provider, fillers::*};
use alloy_rpc_types::{Filter, Log, TransactionInput, TransactionRequest};
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
    primitive::{
        ANGSTROM_ADDRESS, CONTROLLER_V1_ADDRESS, POOL_MANAGER_ADDRESS, POSITION_MANAGER_ADDRESS,
        PoolId
    }
};
use futures::StreamExt;
use lib_reth::{
    DualRethNodeClient, EthApiServer,
    reth_libmdbx::{NodeClientSpec, RethNodeClient},
    traits::EthRevm
};
use pade::PadeDecode;
use reth_provider::BlockNumReader;
use revm::{ExecuteEvm, context::TxEnv};
use uni_v4::{
    BaselinePoolState, FeeConfiguration, PoolKey as UniPoolKey,
    liquidity_base::BaselineLiquidity,
    pool_data_loader::{DataLoader, PoolDataLoader},
    sqrt_pricex96::SqrtPriceX96
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
    apis::{
        AngstromDataApi, AngstromUserApi,
        utils::{
            historical_pool_manager_modify_liquidity_filter, historical_pool_manager_swap_filter
        }
    },
    types::{
        fees::{LiquidityPositionFees, position_fees},
        *
    }
};

#[derive(Clone)]
pub struct RethDbProviderWrapper<Node, P, N>
where
    Node: NodeClientSpec,
    P: Provider<N> + Clone,
    N: Network
{
    provider: DualRethNodeClient<Node, P, N>
}

impl<Node, P, N> RethDbProviderWrapper<Node, P, N>
where
    Node: NodeClientSpec,
    P: Provider<N> + Clone,
    N: Network
{
    pub fn new(provider: DualRethNodeClient<Node, P, N>) -> Self {
        Self { provider }
    }

    pub fn replace_provider(&mut self, provider: P) {
        self.provider.replace_rpc_provider(provider);
    }

    pub fn db_client(&self) -> Arc<RethNodeClient<Node>> {
        self.provider.node_client()
    }

    pub fn rpc_provider(&self) -> P {
        self.provider.rpc_provider()
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
        Ok(self.provider.rpc_provider().get_logs(filter).await?)
    }
}

#[async_trait::async_trait]
impl<Node, P, N> AngstromDataApi for RethDbProviderWrapper<Node, P, N>
where
    Node: NodeClientSpec,
    P: Provider<N> + Clone,
    N: Network<TransactionRequest = TransactionRequest> + RecommendedFillers
{
    async fn tokens_by_partial_pool_key(
        &self,
        pool_partial_key: AngstromPoolPartialKey,
        block_number: Option<u64>
    ) -> eyre::Result<TokenPair> {
        let out = reth_db_view_call(
            &self.db_client(),
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
    ) -> eyre::Result<(u64, BaselinePoolState)> {
        let (token0, token1) = sort_tokens(token0, token1);

        let pool_key = self
            .pool_key_by_tokens(token0, token1, block_number)
            .await?;

        let pool_partial_key = AngstromPoolConfigStore::derive_store_key(token0, token1);

        let uni_pool_key = UniPoolKey {
            currency0:   pool_key.pool_key.currency0,
            currency1:   pool_key.pool_key.currency1,
            fee:         pool_key.pool_fee_in_e6,
            tickSpacing: pool_key.pool_key.tickSpacing,
            hooks:       pool_key.pool_key.hooks
        };

        let public_pool_id: PoolId = PoolId::from(uni_pool_key);
        let mut private_pool_key = uni_pool_key;
        private_pool_key.fee = U24::from(0x800000);
        let private_pool_id: PoolId = PoolId::from(private_pool_key);
        let registry = vec![uni_pool_key].into();

        let data_loader = DataLoader::new_with_registry(
            private_pool_id,
            public_pool_id,
            registry,
            *POOL_MANAGER_ADDRESS.get().unwrap()
        );

        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            lib_reth::helpers::EthApiSpec::chain_info(&self.db_client().eth_api())?.best_number
        };

        let pool_data = data_loader
            .load_pool_data(Some(block_number), Arc::new(self.provider.as_provider_with_db_layer()))
            .await?;

        let fee_config = fetch_fee_configuration(
            &self.provider.rpc_provider(),
            pool_partial_key,
            pool_key.pool_fee_in_e6,
            block_number
        )
        .await?;

        let baseline_liquidity = BaselineLiquidity::new(
            pool_data.tickSpacing.as_i32(),
            pool_data.tick.as_i32(),
            SqrtPriceX96::from(pool_data.sqrtPrice),
            pool_data.liquidity,
            HashMap::new(),
            HashMap::new()
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

        Ok((block_number, baseline_state))
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>
    ) -> eyre::Result<AngstromPoolConfigStore> {
        AngstromPoolConfigStore::load_from_chain(
            *ANGSTROM_ADDRESS.get().unwrap(),
            block_number.map(Into::into).unwrap_or(BlockId::latest()),
            &self.provider.as_provider_with_db_layer()
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
            &self.provider,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
            pool_id,
            block_number
        )
        .await?)
    }

    async fn get_bundle_by_block(
        &self,
        block_number: u64,
        verify_successful_tx: bool
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>> {
        let Some(block) = self
            .db_client()
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
                    transaction.tx_hash(),
                    WithEthMeta::new(
                        transaction.block_number(),
                        Some(transaction.tx_hash()),
                        transaction.transaction_index(),
                        AngstromBundle::pade_decode(&mut input, None).ok()?
                    )
                ))
            });

        if verify_successful_tx {
            let bundles =
                futures::future::try_join_all(angstrom_bundles.map(async |(tx_hash, bundle)| {
                    if self
                        .db_client()
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
            .db_client()
            .eth_api()
            .transaction_by_hash(tx_hash)
            .await?
        else {
            return Ok(None)
        };

        if verify_successful_tx
            && !self
                .db_client()
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
                    transaction.block_number(),
                    Some(transaction.tx_hash()),
                    transaction.transaction_index(),
                    AngstromBundle::pade_decode(&mut input, None).ok()?
                ))
            }))
    }
}

#[async_trait::async_trait]
impl<Node, P, N> AngstromUserApi for RethDbProviderWrapper<Node, P, N>
where
    Node: NodeClientSpec,
    P: Provider<N> + Clone,
    N: Network<TransactionRequest = TransactionRequest> + RecommendedFillers
{
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)> {
        let (pool_key, position_info) = position_manager_pool_key_and_info(
            &self.db_client(),
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
            &self.db_client(),
            *POSITION_MANAGER_ADDRESS.get().unwrap(),
            block_number,
            position_token_id
        )
        .await?;

        let liquidity = pool_manager_position_state_liquidity(
            &self.db_client(),
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

        if start_token_id == U256::ZERO {
            start_token_id = U256::from(1u8);
        }

        if end_token_id == U256::ZERO {
            end_token_id = position_manager_next_token_id(
                &self.db_client(),
                position_manager_address,
                block_number
            )
            .await?;
        }

        let mut all_positions = Vec::new();
        while start_token_id <= end_token_id {
            let owner_of = position_manager_owner_of(
                &self.db_client(),
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
                &self.db_client(),
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
                &self.db_client(),
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
            &self.db_client(),
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

async fn fetch_fee_configuration<P, N>(
    provider: &P,
    pool_partial_key: AngstromPoolPartialKey,
    bundle_fee: U24,
    block_number: u64
) -> eyre::Result<FeeConfiguration>
where
    P: Provider<N> + Clone,
    N: Network<TransactionRequest = TransactionRequest>
{
    const UNLOCKED_FEES_SLOT: u64 = 2;

    let mut preimage = [0u8; 64];
    let key_bytes: &[u8; 27] = pool_partial_key.deref();
    preimage[..27].copy_from_slice(key_bytes);
    preimage[32..].copy_from_slice(&U256::from(UNLOCKED_FEES_SLOT).to_be_bytes::<32>());
    let slot = keccak256(preimage);

    let call = Angstrom::extsloadCall { slot: U256::from_be_bytes(*slot) };
    let tx = TransactionRequest {
        to: Some(TxKind::Call(*ANGSTROM_ADDRESS.get().unwrap())),
        input: TransactionInput::both(call.abi_encode().into()),
        ..Default::default()
    };

    let data = provider.call(tx).block(block_number.into()).await?;

    let raw = Angstrom::extsloadCall::abi_decode_returns(&data)?;

    let bytes = raw.to_be_bytes::<32>();
    let unlocked_fee = U24::from_be_bytes([bytes[29], bytes[30], bytes[31]]);
    let protocol_fee = U24::from_be_bytes([bytes[26], bytes[27], bytes[28]]);

    Ok(FeeConfiguration {
        bundle_fee:   bundle_fee.to::<u32>(),
        swap_fee:     unlocked_fee.to::<u32>(),
        protocol_fee: protocol_fee.to::<u32>()
    })
}

pub(crate) fn reth_db_view_call<Node, IC>(
    provider: &RethNodeClient<Node>,
    block_number: Option<u64>,
    contract: Address,
    call: IC
) -> eyre::Result<Result<IC::Return, alloy_sol_types::Error>>
where
    Node: NodeClientSpec,
    IC: SolCall + Send
{
    let tx = TxEnv {
        kind: TxKind::Call(contract),
        data: call.abi_encode().into(),
        ..Default::default()
    };

    let block_number = if let Some(bn) = block_number {
        bn
    } else {
        provider.eth_db_provider().best_block_number()?
    };

    let mut evm = provider.make_empty_evm(block_number)?;

    let data = evm.transact(tx)?;

    Ok(IC::abi_decode_returns(data.result.output().unwrap_or_default()))
}
