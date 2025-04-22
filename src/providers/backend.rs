use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::{
    Address, FixedBytes, PrimitiveSignature, TxHash, U256,
    aliases::{I24, U24},
};
use alloy_provider::{
    Identity, Provider, RootProvider,
    fillers::{
        BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
    },
};
use alloy_signer::{Signer, SignerSync};

use angstrom_types::{
    contract_bindings::{
        angstrom::Angstrom::PoolKey, controller_v_1::ControllerV1,
        mintable_mock_erc_20::MintableMockERC20, position_fetcher::PositionFetcher,
        position_manager::PositionManager,
    },
    primitive::PoolId,
};
use futures::{StreamExt, TryFutureExt};
use jsonrpsee_http_client::HttpClient;
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    apis::{
        data_api::AngstromDataApi,
        node_api::AngstromNodeApi,
        user_api::AngstromUserApi,
        utils::{pool_config_store, view_call},
    },
    types::*,
};

pub type AlloyRpcProvider<P> = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    P,
>;

pub type AlloyWalletRpcProvider<P> =
    FillProvider<JoinFill<Identity, WalletFiller<EthereumWallet>>, P>;

#[derive(Debug, Clone)]
pub struct AngstromProvider<P>
where
    P: Provider,
{
    eth_provider: P,
    angstrom_provider: HttpClient,
}

impl AngstromProvider<AlloyRpcProvider<RootProvider>> {
    pub async fn new(eth_url: &str, angstrom_url: &str) -> eyre::Result<Self> {
        let angstrom_provider = HttpClient::builder().build(angstrom_url)?;
        Ok(Self {
            eth_provider: RootProvider::builder()
                .with_recommended_fillers()
                .connect(eth_url)
                .await?,
            angstrom_provider,
        })
    }
}
impl<P: Provider> AngstromProvider<P> {
    pub fn new_with_provider(eth_provider: P, angstrom_url: &str) -> eyre::Result<Self> {
        let angstrom_provider = HttpClient::builder().build(angstrom_url.to_string())?;
        Ok(Self { eth_provider, angstrom_provider })
    }

    pub fn eth_provider(&self) -> &P {
        &self.eth_provider
    }

    pub async fn send_add_remove_liquidity_tx(
        &self,
        tx_req: TransactionRequestWithLiquidityMeta,
    ) -> eyre::Result<TxHash> {
        Ok(self
            .eth_provider
            .send_transaction(tx_req.tx_request)
            .await?
            .watch()
            .await?)
    }

    pub(crate) fn with_wallet<S>(self, signer: S) -> AngstromProvider<AlloyWalletRpcProvider<P>>
    where
        S: Signer + SignerSync + TxSigner<PrimitiveSignature> + Send + Sync + 'static,
    {
        let eth_provider = alloy_provider::builder::<Ethereum>()
            .wallet(EthereumWallet::new(signer))
            .on_provider(self.eth_provider);

        AngstromProvider { eth_provider, angstrom_provider: self.angstrom_provider }
    }
}

impl<P> AngstromDataApi for AngstromProvider<P>
where
    P: Provider,
{
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        self.eth_provider.all_token_pairs().await
    }

    async fn all_tokens(&self) -> eyre::Result<Vec<TokenInfoWithMeta>> {
        self.eth_provider.all_tokens().await
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        self.eth_provider.pool_key(token0, token1).await
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        self.eth_provider.historical_orders(filter).await
    }

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        self.eth_provider
            .pool_data(token0, token1, block_number)
            .await
    }
}

impl<P: Provider> AngstromUserApi for AngstromProvider<P> {
    async fn get_positions(
        &self,
        user_address: Address,
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        self.eth_provider.get_positions(user_address).await
    }
}

impl<P: Provider> AngstromNodeApi for AngstromProvider<P> {
    fn angstrom_rpc_provider(&self) -> HttpClient {
        self.angstrom_provider.clone()
    }
}

impl<P: Provider> AngstromDataApi for P {
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        let config_store = pool_config_store(self).await?;
        let partial_key_entries = config_store.all_entries();

        let all_pools_call = futures::future::try_join_all(partial_key_entries.iter().map(|key| {
            view_call(
                self,
                CONTROLLER_V1_ADDRESS,
                ControllerV1::poolsCall { key: FixedBytes::from(*key.pool_partial_key) },
            )
        }))
        .await?;

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

        Ok(futures::future::try_join_all(all_tokens_addresses.into_iter().map(|address| {
            view_call(self, address, MintableMockERC20::symbolCall {}).and_then(
                async move |val_res| {
                    Ok(val_res.map(|val| TokenInfoWithMeta { address, symbol: val._0 }))
                },
            )
        }))
        .await?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?)
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        let (token0, token1) = sort_tokens(token0, token1);

        let config_store = pool_config_store(self).await?;
        let pool_config_store = config_store
            .get_entry(token0, token1)
            .ok_or(eyre::eyre!("no config store entry for tokens {token0:?} - {token1:?}"))?;

        Ok(PoolKey {
            currency0: token0,
            currency1: token1,
            fee: U24::from(pool_config_store.fee_in_e6),
            tickSpacing: I24::unchecked_from(pool_config_store.tick_spacing),
            hooks: ANGSTROM_ADDRESS,
        })
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        let filter = &filter;
        let pool_stores = &AngstromPoolTokenIndexToPair::new_with_tokens(self, filter).await?;

        let start_block = filter.from_block.unwrap_or(ANGSTROM_DEPLOYED_BLOCK);
        let end_block =
            if let Some(e) = filter.to_block { e } else { self.get_block_number().await? };

        let mut block_stream = futures::stream::iter(start_block..end_block)
            .map(|bn| async move {
                let block = self
                    .get_block(bn.into())
                    .full()
                    .await?
                    .ok_or(eyre::eyre!("block number {bn} not found"))?;

                Ok::<_, eyre::ErrReport>(filter.filter_block(block, pool_stores))
            })
            .buffer_unordered(10);

        let mut all_orders = Vec::new();
        while let Some(val) = block_stream.next().await {
            all_orders.extend(val?);
        }

        Ok(all_orders)
    }

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        let (token0, token1) = sort_tokens(token0, token1);

        let mut pool_key = self.pool_key(token0, token1).await?;
        pool_key.fee = U24::from(0x800000);
        let pool_id: PoolId = pool_key.clone().into();

        let data_loader =
            DataLoader::new_with_registry(pool_id, vec![pool_key].into(), POOL_MANAGER_ADDRESS);

        let mut enhanced_uni_pool = EnhancedUniswapPool::new(data_loader, 200);

        let block_number =
            if let Some(bn) = block_number { bn } else { self.get_block_number().await? };

        enhanced_uni_pool
            .initialize(Some(block_number), Arc::new(self))
            .await?;

        Ok((block_number, enhanced_uni_pool))
    }
}

impl<P: Provider> AngstromUserApi for P {
    async fn get_positions(
        &self,
        user_address: Address,
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        let user_positons = view_call(
            self,
            POSITION_MANAGER_ADDRESS,
            PositionFetcher::getPositionsCall {
                owner: user_address,
                tokenId: U256::from(1u8),
                lastTokenId: U256::ZERO,
                maxResults: U256::MAX,
            },
        )
        .await??;

        let unique_pool_ids = user_positons
            ._2
            .iter()
            .map(|pos: &PositionFetcher::Position| pos.poolId)
            .collect::<HashSet<_>>();

        let uni_pool_id_to_ang_pool_ids =
            futures::future::try_join_all(unique_pool_ids.into_iter().map(|uni_id| {
                view_call(
                    self,
                    POSITION_MANAGER_ADDRESS,
                    PositionManager::poolKeysCall { poolId: uni_id },
                )
                .and_then(async move |ang_id_res| {
                    Ok(ang_id_res.map(|ang_id| {
                        (
                            uni_id,
                            PoolKey {
                                currency0: ang_id.currency0,
                                currency1: ang_id.currency1,
                                fee: ang_id.fee,
                                tickSpacing: ang_id.tickSpacing,
                                hooks: ang_id.hooks,
                            },
                        )
                    }))
                })
            }))
            .await?
            .into_iter()
            .collect::<Result<HashMap<_, _>, _>>()?;

        Ok(user_positons
            ._2
            .into_iter()
            .map(|pos| {
                UserLiquidityPosition::new(
                    uni_pool_id_to_ang_pool_ids
                        .get(&pos.poolId)
                        .unwrap()
                        .clone(),
                    pos,
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod data_api_tests {

    use crate::test_utils::spawn_angstrom_api;

    use super::*;

    #[tokio::test]
    async fn test_all_token_pairs() {
        let provider = spawn_angstrom_api().await.unwrap();

        let all_pairs = provider.all_token_pairs().await.unwrap();
        assert!(!all_pairs.is_empty());

        let contains = all_pairs
            .into_iter()
            .any(|pair| USDC == pair.token0 && WETH == pair.token1);
        assert!(contains);
    }

    #[tokio::test]
    async fn test_all_tokens() {
        let provider = spawn_angstrom_api().await.unwrap();

        let pool_keys = provider.all_tokens().await.unwrap();
        assert!(!pool_keys.is_empty());

        let contains_usdc = pool_keys
            .iter()
            .any(|token| token.address == USDC && &token.symbol == "USDC");
        let contains_weth = pool_keys
            .iter()
            .any(|token| token.address == WETH && &token.symbol == "WETH");

        assert!(contains_usdc);
        assert!(contains_weth);
    }

    #[tokio::test]
    async fn test_pool_key() {
        let provider = spawn_angstrom_api().await.unwrap();
        let token0 = USDC;
        let token1 = WETH;

        let pool_key = provider.pool_key(token0, token1).await.unwrap();
        let expected_pool_key = PoolKey {
            currency0: token0,
            currency1: token1,
            fee: U24::ZERO,
            tickSpacing: I24::unchecked_from(30),
            hooks: ANGSTROM_ADDRESS,
        };

        assert_eq!(pool_key, expected_pool_key);
    }

    #[tokio::test]
    async fn test_historical_orders() {
        todo!()
    }

    #[tokio::test]
    async fn test_pool_data() {
        todo!()
    }
}

#[cfg(test)]
mod user_tests {
    // use alloy_primitives::address;

    // use super::*;
    // use crate::test_utils::spawn_angstrom_api;

    #[tokio::test]
    async fn test_get_positions() {
        // let angstrom_api = spawn_angstrom_api().await.unwrap();

        // let positions = angstrom_api
        //     .get_positions(address!("0x796fB50EAe1456A523F869f6135dd557eeaEE226"))
        //     .await
        //     .unwrap();

        // println!("{positions:?}");

        todo!()
    }

    #[tokio::test]
    async fn test_get_positions_in_pool() {
        todo!()
    }
}
