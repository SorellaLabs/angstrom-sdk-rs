use crate::apis::AngstromDataApi;
use crate::types::*;
use alloy_consensus::Transaction;
use alloy_eips::BlockId;
use alloy_network::Ethereum;
use alloy_primitives::TxKind;
use alloy_primitives::{
    Address, FixedBytes,
    aliases::{I24, U24},
};
use alloy_provider::Identity;
use alloy_provider::Provider;
use alloy_provider::ProviderBuilder;
use angstrom_types::reth_db_provider::RethDbLayer;
use angstrom_types::reth_db_provider::RethDbProvider;
use lib_reth::EthApiServer;
use lib_reth::traits::EthRevm;

use alloy_provider::fillers::*;
use alloy_sol_types::SolCall;
use angstrom_types::{
    contract_bindings::{
        angstrom::Angstrom::{PoolKey, executeCall},
        controller_v_1::ControllerV1::getPoolByKeyCall,
        mintable_mock_erc_20::MintableMockERC20,
    },
    contract_payloads::angstrom::{AngstromBundle, AngstromPoolConfigStore},
    primitive::{
        ANGSTROM_ADDRESS, ANGSTROM_DEPLOYED_BLOCK, CONTROLLER_V1_ADDRESS, POOL_MANAGER_ADDRESS,
        PoolId,
    },
};
use reth_db::DatabaseEnv;
use reth_node_ethereum::EthereumNode;
use reth_node_types::NodeTypesWithDBAdapter;
use reth_provider::providers::BlockchainProvider;
use revm::ExecuteEvm;
use revm::context::TxEnv;
use std::{collections::HashSet, sync::Arc};

use futures::StreamExt;
use lib_reth::reth_libmdbx::RethLibmdbxClient;
use pade::PadeDecode;
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

pub type RethLayerProviderWrapperType<P> = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    RethDbProvider<P, BlockchainProvider<NodeTypesWithDBAdapter<EthereumNode, Arc<DatabaseEnv>>>>,
>;

pub struct RethDbProviderWrapper<P: Provider + Clone> {
    db_client: Arc<RethLibmdbxClient>,
    provider: P,
}

impl<P: Provider + Clone> RethDbProviderWrapper<P> {
    pub fn new(db_client: RethLibmdbxClient, provider: P) -> eyre::Result<Self> {
        Ok(Self { db_client: Arc::new(db_client), provider })
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

impl<P: Provider + Clone> AngstromDataApi for RethDbProviderWrapper<P> {
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        let config_store = self.pool_config_store(None).await?;
        let partial_key_entries = config_store.all_entries();

        let all_pools_call = partial_key_entries
            .iter()
            .map(|key| {
                reth_db_view_call(
                    &self.db_client,
                    *CONTROLLER_V1_ADDRESS.get().unwrap(),
                    getPoolByKeyCall { key: FixedBytes::from(*key.pool_partial_key) },
                )
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(all_pools_call
            .into_iter()
            .map(|val_res| {
                val_res.map(|val| TokenPairInfo {
                    token0: val.asset0,
                    token1: val.asset1,
                    is_active: true,
                })
            })
            .collect::<Result<Vec<_>, _>>()?)
    }

    async fn all_tokens(&self) -> eyre::Result<Vec<TokenInfoWithMeta>> {
        let all_tokens_addresses = self
            .all_token_pairs()
            .await?
            .into_iter()
            .flat_map(|val| [val.token0, val.token1])
            .collect::<HashSet<_>>();

        Ok(all_tokens_addresses
            .into_iter()
            .map(|address| {
                reth_db_view_call(&self.db_client, address, MintableMockERC20::symbolCall {})
                    .map(|val_res| val_res.map(|val| TokenInfoWithMeta { address, symbol: val }))
            })
            .collect::<Result<Result<Vec<_>, _>, _>>()??)
    }

    async fn pool_key(
        &self,
        token0: Address,
        token1: Address,
        uniswap_key: bool,
        block_number: Option<u64>,
    ) -> eyre::Result<PoolKey> {
        let (token0, token1) = sort_tokens(token0, token1);

        let config_store = self.pool_config_store(block_number).await?;
        let pool_config_store = config_store
            .get_entry(token0, token1)
            .ok_or(eyre::eyre!("no config store entry for tokens {token0:?} - {token1:?}"))?;

        Ok(PoolKey {
            currency0: token0,
            currency1: token1,
            fee: if uniswap_key {
                U24::from(8388608u32)
            } else {
                U24::from(pool_config_store.fee_in_e6)
            },
            tickSpacing: I24::unchecked_from(pool_config_store.tick_spacing),
            hooks: *ANGSTROM_ADDRESS.get().unwrap(),
        })
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
        block_stream_buffer: Option<usize>,
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
        block_stream_buffer: Option<usize>,
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
                        }),
                )
            })
            .buffer_unordered(block_stream_buffer.unwrap_or(10));

        let mut all_bundles = Vec::new();
        while let Some(val) = block_stream.next().await {
            all_bundles.extend(val?);
        }

        Ok(all_bundles)
    }

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        let (token0, token1) = sort_tokens(token0, token1);

        let mut pool_key = self.pool_key(token0, token1, false, block_number).await?;
        let public_pool_id = pool_key.clone().into();
        let registry = vec![pool_key.clone()].into();

        pool_key.fee = U24::from(0x800000);
        let private_pool_id: PoolId = pool_key.clone().into();

        let data_loader = DataLoader::new_with_registry(
            private_pool_id,
            public_pool_id,
            registry,
            *POOL_MANAGER_ADDRESS.get().unwrap(),
        );

        let mut enhanced_uni_pool = EnhancedUniswapPool::new(data_loader, 200);

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
        block_number: Option<u64>,
    ) -> eyre::Result<AngstromPoolConfigStore> {
        AngstromPoolConfigStore::load_from_chain(
            *ANGSTROM_ADDRESS.get().unwrap(),
            block_number.map(Into::into).unwrap_or(BlockId::latest()),
            &self.as_provider_with_db_layer(),
        )
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))
    }
}

pub(crate) fn reth_db_view_call<IC>(
    provider: &RethLibmdbxClient,
    contract: Address,
    call: IC,
) -> eyre::Result<Result<IC::Return, alloy_sol_types::Error>>
where
    IC: SolCall + Send,
{
    let tx = TxEnv {
        kind: TxKind::Call(contract),
        data: call.abi_encode().into(),
        ..Default::default()
    };

    let mut evm = provider.make_empty_evm(provider.eth_api().block_number()?.to())?;

    let data = evm.transact(tx)?;

    Ok(IC::abi_decode_returns(&*data.result.output().unwrap_or_default()))
}
