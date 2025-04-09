use std::{collections::HashSet, sync::Arc};

use alloy::transports::TransportErrorKind;
use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::{
    Address, FixedBytes, PrimitiveSignature, TxHash, TxKind,
    aliases::{I24, U24},
};
use alloy_provider::{
    Identity, Provider, RootProvider,
    fillers::{FillProvider, JoinFill, WalletFiller},
};
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_signer::{Signer, SignerSync};
use alloy_sol_types::SolCall;
use alloy_transport::RpcError;
use angstrom_types::{
    contract_bindings::{
        angstrom::Angstrom::PoolKey, controller_v_1::ControllerV1,
        mintable_mock_erc_20::MintableMockERC20,
    },
    primitive::PoolId,
};
use futures::{StreamExt, TryFutureExt};
use jsonrpsee_http_client::HttpClient;
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    FillWrapper, FillerOrder,
    apis::{data_api::AngstromDataApi, node_api::AngstromNodeApi, utils::pool_config_store},
    types::*,
};

pub(crate) type RpcWalletProvider<P> =
    FillProvider<JoinFill<Identity, WalletFiller<EthereumWallet>>, P, Ethereum>;

#[derive(Debug, Clone)]
pub struct AngstromProvider<P>
where
    P: Provider,
{
    eth_provider: P,
    angstrom_provider: HttpClient,
}

impl AngstromProvider<RootProvider> {
    pub async fn new(eth_url: &str, angstrom_url: &str) -> eyre::Result<Self> {
        let angstrom_provider = HttpClient::builder().build(angstrom_url)?;
        Ok(Self {
            eth_provider: RootProvider::builder().connect(eth_url).await?,
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

    pub(crate) async fn view_call<IC>(
        &self,
        contract: Address,
        call: IC,
    ) -> Result<Result<IC::Return, alloy_sol_types::Error>, RpcError<TransportErrorKind>>
    where
        IC: SolCall + Send,
    {
        let tx = TransactionRequest {
            to: Some(TxKind::Call(contract)),
            input: TransactionInput::both(call.abi_encode().into()),
            ..Default::default()
        };

        let data = self.eth_provider().call(tx).await?;
        Ok(IC::abi_decode_returns(&data, false))
    }

    pub(crate) fn with_wallet<S>(self, signer: S) -> AngstromProvider<RpcWalletProvider<P>>
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
    P: Provider + Clone,
{
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        let config_store = pool_config_store(self.eth_provider()).await?;
        let partial_key_entries = config_store.all_entries();

        let all_pools_call = futures::future::try_join_all(partial_key_entries.iter().map(|key| {
            self.view_call(
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
            self.view_call(address, MintableMockERC20::symbolCall {})
                .and_then(async move |val_res| {
                    Ok(val_res.map(|val| TokenInfoWithMeta { address, symbol: val._0 }))
                })
        }))
        .await?
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?)
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        let (token0, token1) = sort_tokens(token0, token1);

        let config_store = pool_config_store(self.eth_provider()).await?;
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
        let end_block = if let Some(e) = filter.to_block {
            e
        } else {
            self.eth_provider.get_block_number().await?
        };

        let mut block_stream = futures::stream::iter(start_block..end_block)
            .map(|bn| async move {
                let block = self
                    .eth_provider
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
    ) -> eyre::Result<EnhancedUniswapPool<DataLoader>> {
        let (token0, token1) = sort_tokens(token0, token1);

        let mut pool_key = self.pool_key(token0, token1).await?;
        pool_key.fee = U24::from(0x800000);
        let pool_id: PoolId = pool_key.clone().into();

        let data_loader =
            DataLoader::new_with_registry(pool_id, vec![pool_key].into(), POOL_MANAGER_ADDRESS);

        let mut enhanced_uni_pool = EnhancedUniswapPool::new(data_loader, 200);

        let block_number = if let Some(bn) = block_number {
            bn
        } else {
            self.eth_provider.get_block_number().await?
        };

        enhanced_uni_pool
            .initialize(Some(block_number), Arc::new(self.eth_provider.clone()))
            .await?;

        Ok(enhanced_uni_pool)
    }
}

impl<P: Provider> AngstromNodeApi for AngstromProvider<P> {
    fn angstrom_rpc_provider(&self) -> HttpClient {
        self.angstrom_provider.clone()
    }
}

#[cfg(test)]
mod tests {

    use alloy_primitives::address;

    use crate::test_utils::spawn_angstrom_api;

    use super::*;

    #[tokio::test]
    async fn test_all_token_pairs() {
        let provider = spawn_angstrom_api().await.unwrap();

        let all_pairs = provider.all_token_pairs().await.unwrap();

        assert!(!all_pairs.is_empty());
        let first = all_pairs.first().unwrap();

        assert_ne!(Address::ZERO, first.token0);
        assert_ne!(Address::ZERO, first.token1);
    }

    #[tokio::test]
    async fn test_pool_key() {
        let provider = spawn_angstrom_api().await.unwrap();
        let token0 = address!("2260fac5e5542a773aa44fbcfedf7c193bc2c599");
        let token1 = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");

        let pool_key = provider.pool_key(token0, token1).await.unwrap();
        let expected_pool_key = PoolKey {
            currency0: token0,
            currency1: token1,
            fee: U24::ZERO,
            tickSpacing: I24::unchecked_from(60),
            hooks: ANGSTROM_ADDRESS,
        };

        assert_eq!(pool_key, expected_pool_key);
    }
}
