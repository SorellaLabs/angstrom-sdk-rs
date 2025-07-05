use std::sync::Arc;

use alloy_consensus::Transaction;
use alloy_eips::BlockId;
use alloy_network::Ethereum;
use alloy_primitives::{
    Address, FixedBytes, TxKind,
    aliases::{I24, U24}
};
use alloy_provider::{Identity, Provider, ProviderBuilder, fillers::*};
use alloy_sol_types::SolCall;
use angstrom_types::{
    contract_bindings::{
        angstrom::Angstrom::{PoolKey, executeCall},
        controller_v_1::ControllerV1
    },
    contract_payloads::angstrom::{
        AngstromBundle, AngstromPoolConfigStore, AngstromPoolPartialKey
    },
    primitive::{
        ANGSTROM_ADDRESS, ANGSTROM_DEPLOYED_BLOCK, CONTROLLER_V1_ADDRESS, POOL_MANAGER_ADDRESS,
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
    apis::AngstromDataApi,
    types::{
        positions::{pool_manager_pool_slot0, utils::UnpackedSlot0},
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

    pub fn as_provider_with_db_layer(&self) -> RethLayerProviderWrapperType<P> {
        ProviderBuilder::<_, _, Ethereum>::default()
            .with_recommended_fillers()
            .layer(RethDbLayer::new(self.db_client.eth_db_provider().clone()))
            .connect_provider(self.provider.clone())
    }

    pub fn db_client(&self) -> Arc<RethLibmdbxClient> {
        self.db_client.clone()
    }
}

#[async_trait::async_trait]
impl<P: Provider + Clone> AngstromDataApi for RethDbProviderWrapper<P> {
    async fn tokens_by_partial_pool_key(
        &self,
        pool_partial_key: AngstromPoolPartialKey,
        block_number: Option<u64>
    ) -> eyre::Result<TokenPairInfo> {
        let out = reth_db_view_call(
            &self.db_client,
            block_number,
            *CONTROLLER_V1_ADDRESS.get().unwrap(),
            ControllerV1::getPoolByKeyCall { key: FixedBytes::from(*pool_partial_key) }
        )??;

        Ok(TokenPairInfo { token0: out.asset0, token1: out.asset1 })
    }

    async fn all_token_pairs_with_config_store(
        &self,
        block_number: Option<u64>,
        config_store: AngstromPoolConfigStore
    ) -> eyre::Result<Vec<TokenPairInfo>> {
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

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        let filter = &filter;
        let pool_stores = &AngstromPoolTokenIndexToPair::new_with_tokens(self, filter).await?;

        let start_block = filter
            .from_block
            .unwrap_or(*ANGSTROM_DEPLOYED_BLOCK.get().unwrap());
        let end_block = if let Some(e) = filter.to_block {
            e
        } else {
            self.db_client.eth_api().block_number()?.to()
        };

        let mut block_stream = futures::stream::iter(start_block..=end_block)
            .map(|bn| async move {
                let block = self
                    .db_client
                    .eth_api()
                    .block_by_number(bn.into(), true)
                    .await?
                    .ok_or(eyre::eyre!("block number {bn} not found"))?;

                Ok::<_, eyre::ErrReport>(filter.filter_block(block, pool_stores))
            })
            .buffer_unordered(block_stream_buffer.unwrap_or(10));

        let mut all_orders = Vec::new();
        while let Some(val) = block_stream.next().await {
            all_orders.extend(val?);
        }

        Ok(all_orders)
    }

    async fn historical_bundles(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<AngstromBundle>> {
        let start_block = start_block.unwrap_or(*ANGSTROM_DEPLOYED_BLOCK.get().unwrap());
        let end_block =
            if let Some(e) = end_block { e } else { self.db_client.eth_api().block_number()?.to() };

        let mut block_stream = futures::stream::iter(start_block..=end_block)
            .map(|bn| async move {
                let block = self
                    .db_client
                    .eth_api()
                    .block_by_number(bn.into(), true)
                    .await?
                    .ok_or(eyre::eyre!("block number {bn} not found"))?;

                Ok::<_, eyre::ErrReport>(
                    block
                        .transactions
                        .into_transactions()
                        .filter(|tx| tx.to() == Some(*ANGSTROM_ADDRESS.get().unwrap()))
                        .filter_map(|transaction| {
                            let input: &[u8] = transaction.input();
                            let call = executeCall::abi_decode(input).ok()?;
                            let mut input = call.encoded.as_ref();
                            AngstromBundle::pade_decode(&mut input, None).ok()
                        })
                )
            })
            .buffer_unordered(block_stream_buffer.unwrap_or(10));

        let mut all_bundles = Vec::new();
        while let Some(val) = block_stream.next().await {
            all_bundles.extend(val?);
        }

        Ok(all_bundles)
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

    async fn pool_slot0(
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

    Ok(IC::abi_decode_returns(&*data.result.output().unwrap_or_default()))
}
