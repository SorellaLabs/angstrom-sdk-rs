use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use alloy_consensus::Transaction;
use alloy_eips::BlockId;
use alloy_primitives::{
    Address, FixedBytes,
    aliases::{I24, U24},
};
use alloy_provider::Provider;
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
use futures::{FutureExt, StreamExt, TryFutureExt};
use pade::PadeDecode;
use uniswap_v4::uniswap::{
    pool::EnhancedUniswapPool, pool_data_loader::DataLoader, pool_factory::INITIAL_TICKS_PER_SIDE,
};

use super::utils::*;
use crate::types::*;

#[async_trait::async_trait]
pub trait AngstromDataApi: Send {
    async fn all_token_pairs(&self, block_number: Option<u64>) -> eyre::Result<Vec<TokenPairInfo>>;

    async fn all_tokens(&self, block_number: Option<u64>) -> eyre::Result<Vec<TokenInfoWithMeta>>;

    async fn pool_key(
        &self,
        token0: Address,
        token1: Address,
        uniswap_key: bool,
        block_number: Option<u64>,
    ) -> eyre::Result<PoolKey>;

    async fn all_pool_keys(
        &self,
        uniswap_key: bool,
        block_number: Option<u64>,
    ) -> eyre::Result<Vec<PoolKey>> {
        let (config_store, all_token_pairs) = tokio::try_join!(
            self.pool_config_store(block_number),
            self.all_token_pairs(block_number)
        )?;

        let tokens_to_partial_keys = all_token_pairs
            .into_iter()
            .map(|tokens| {
                (
                    AngstromPoolConfigStore::derive_store_key(tokens.token0, tokens.token1),
                    (tokens.token0, tokens.token1),
                )
            })
            .collect::<HashMap<_, _>>();

        Ok(config_store
            .all_entries()
            .into_iter()
            .map(|entry| {
                let (k, v) = entry.pair();
                let (token0, token1) = tokens_to_partial_keys.get(k).unwrap();

                PoolKey {
                    currency0: *token0,
                    currency1: *token1,
                    fee: if uniswap_key { U24::from(8388608u32) } else { U24::from(v.fee_in_e6) },
                    tickSpacing: I24::unchecked_from(v.tick_spacing),
                    hooks: *ANGSTROM_ADDRESS.get().unwrap(),
                }
            })
            .collect())
    }

    async fn pool_id(
        &self,
        token0: Address,
        token1: Address,
        uniswap_key: bool,
        block_number: Option<u64>,
    ) -> eyre::Result<PoolId> {
        self.pool_key(token0, token1, uniswap_key, block_number)
            .await
            .map(Into::into)
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
        block_stream_buffer: Option<usize>,
    ) -> eyre::Result<Vec<HistoricalOrders>>;

    async fn historical_bundles(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        block_stream_buffer: Option<usize>,
    ) -> eyre::Result<Vec<AngstromBundle>>;

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)>;

    async fn all_pool_data(
        &self,
        block_number: Option<u64>,
    ) -> eyre::Result<Vec<(u64, EnhancedUniswapPool<DataLoader>)>> {
        let token_pairs = self.all_token_pairs(block_number).await?;

        let pools = futures::future::try_join_all(
            token_pairs
                .into_iter()
                .map(|pair| self.pool_data(pair.token0, pair.token1, block_number)),
        )
        .await?;

        Ok(pools)
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>,
    ) -> eyre::Result<AngstromPoolConfigStore>;
}

#[async_trait::async_trait]
impl<P: Provider> AngstromDataApi for P {
    async fn all_token_pairs(&self, block_number: Option<u64>) -> eyre::Result<Vec<TokenPairInfo>> {
        let config_store = self.pool_config_store(block_number).await?;
        let partial_key_entries = config_store.all_entries();

        let all_pools_call = futures::future::try_join_all(partial_key_entries.iter().map(|key| {
            view_call(
                self,
                *CONTROLLER_V1_ADDRESS.get().unwrap(),
                getPoolByKeyCall { key: FixedBytes::from(*key.pool_partial_key) },
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

    async fn all_tokens(&self, block_number: Option<u64>) -> eyre::Result<Vec<TokenInfoWithMeta>> {
        let all_tokens_addresses = self
            .all_token_pairs(block_number)
            .await?
            .into_iter()
            .flat_map(|val| [val.token0, val.token1])
            .collect::<HashSet<_>>();

        Ok(futures::future::try_join_all(all_tokens_addresses.into_iter().map(|address| {
            view_call(self, address, MintableMockERC20::symbolCall {})
                .and_then(async move |val_res| {
                    Ok(val_res.map(|val| TokenInfoWithMeta { address, symbol: val }))
                })
                .boxed()
        }))
        .await?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?)
    }

    async fn pool_key(
        &self,
        token0: Address,
        token1: Address,
        uniswap_key: bool,
        block_number: Option<u64>,
    ) -> eyre::Result<PoolKey> {
        let (token0, token1) = sort_tokens(token0, token1);

        let config_store = Box::pin(self.pool_config_store(block_number)).await?;
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
        let end_block =
            if let Some(e) = filter.to_block { e } else { self.get_block_number().await? };

        let mut block_stream = futures::stream::iter(start_block..=end_block)
            .map(|bn| async move {
                let block = self
                    .get_block(bn.into())
                    .full()
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
        let end_block = if let Some(e) = end_block { e } else { self.get_block_number().await? };

        let mut block_stream = futures::stream::iter(start_block..=end_block)
            .map(|bn| async move {
                let block = self
                    .get_block(bn.into())
                    .full()
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
        block_number: Option<u64>,
    ) -> eyre::Result<AngstromPoolConfigStore> {
        AngstromPoolConfigStore::load_from_chain(
            *ANGSTROM_ADDRESS.get().unwrap(),
            block_number.map(Into::into).unwrap_or(BlockId::latest()),
            self,
        )
        .await
        .map_err(|e| eyre::eyre!("{e:?}"))
    }
}

#[cfg(test)]
mod tests {

    use alloy_primitives::aliases::{I24, U24};

    use super::*;
    use crate::test_utils::*;

    #[tokio::test]
    async fn test_all_token_pairs() {
        let provider = spawn_angstrom_api().await.unwrap();

        let all_pairs = provider.all_token_pairs(None).await.unwrap();
        assert!(!all_pairs.is_empty());

        let contains = all_pairs
            .into_iter()
            .any(|pair| USDC == pair.token0 && WETH == pair.token1);
        assert!(contains);
    }

    #[tokio::test]
    async fn test_all_tokens() {
        let provider = spawn_angstrom_api().await.unwrap();

        let pool_keys = provider.all_tokens(None).await.unwrap();
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

        let pool_key = provider
            .pool_key(token0, token1, false, None)
            .await
            .unwrap();
        let expected_pool_key = PoolKey {
            currency0: token0,
            currency1: token1,
            fee: U24::ZERO,
            tickSpacing: I24::unchecked_from(30),
            hooks: *ANGSTROM_ADDRESS.get().unwrap(),
        };

        assert_eq!(pool_key, expected_pool_key);
    }

    #[tokio::test]
    async fn test_historical_orders() {
        let provider = spawn_angstrom_api().await.unwrap();
        let filter = HistoricalOrdersFilter::new()
            .from_block(8214200)
            .to_block(8214320)
            .order_kind(OrderKind::User);
        let orders = provider.historical_orders(filter, None).await.unwrap();

        assert_eq!(orders.len(), 5);
    }

    #[tokio::test]
    async fn test_pool_data() {
        let provider = spawn_angstrom_api().await.unwrap();
        // let _ = provider.pool_data(USDC, WETH, None).await.unwrap();

        let all_pools = provider.all_pool_data(None).await.unwrap();

        all_pools
            .into_iter()
            .for_each(|(_, pool)| println!("{:?}\n\n", pool));
    }
}
