use std::sync::Arc;

use alloy::json_abi::Function;
use alloy_dyn_abi::DynSolValue;
use alloy_multicall::Multicall;
use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::{
    address,
    aliases::{I24, U24},
    Address, FixedBytes, PrimitiveSignature, TxHash, TxKind, B256
};
use alloy_provider::{
    fillers::{FillProvider, JoinFill, WalletFiller},
    Identity, Provider, RootProvider
};
use alloy_rpc_types::{BlockTransactionsKind, TransactionInput, TransactionRequest};
use alloy_signer::{Signer, SignerSync};
use alloy_sol_types::{SolCall, SolInterface};
use alloy_transport::BoxTransport;
use angstrom_types::{
    contract_bindings::{
        angstrom::Angstrom::PoolKey,
        controller_v_1::ControllerV1::{self, poolsCall, ControllerV1Calls, ControllerV1Instance},
        mintable_mock_erc_20::MintableMockERC20::{
            self, MintableMockERC20Calls, MintableMockERC20Instance
        }
    },
    primitive::{PoolId, ERC20},
    sol_bindings::testnet::MockERC20::MockERC20Instance
};
use futures::StreamExt;
use serde_json::Value;
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    apis::{data_api::AngstromDataApi, utils::pool_config_store},
    types::*
};

pub(crate) type RpcWalletProvider<P> =
    FillProvider<JoinFill<Identity, WalletFiller<EthereumWallet>>, P, BoxTransport, Ethereum>;

#[derive(Debug, Clone)]
pub struct EthRpcProvider<P>
where
    P: Provider + Clone
{
    eth_provider: P,
    web_provider: reqwest::Client
}

impl EthRpcProvider<RootProvider<BoxTransport>> {
    /// based on the url passed in, will auto parse to http, ws or ipc
    pub async fn new(url: &str) -> eyre::Result<Self> {
        Ok(Self {
            eth_provider: RootProvider::<BoxTransport, _>::builder()
                .on_builtin(url)
                .await?,
            web_provider: reqwest::Client::new()
        })
    }
}
impl<P: Provider + Clone> EthRpcProvider<P> {
    pub fn provider(&self) -> &P {
        &self.eth_provider
    }

    pub(crate) async fn view_call<IC>(
        &self,
        contract: Address,
        call: IC
    ) -> eyre::Result<IC::Return>
    where
        IC: SolCall + Send
    {
        let tx = TransactionRequest {
            to: Some(TxKind::Call(contract)),
            input: TransactionInput::both(call.abi_encode().into()),
            ..Default::default()
        };

        Ok(IC::abi_decode_returns(&self.provider().call(&tx).await?, false)?)
    }

    pub(crate) fn with_wallet<S>(self, signer: S) -> EthRpcProvider<RpcWalletProvider<P>>
    where
        S: Signer + SignerSync + TxSigner<PrimitiveSignature> + Send + Sync + 'static
    {
        let eth_provider = alloy_provider::builder::<Ethereum>()
            .wallet(EthereumWallet::new(signer))
            .on_provider(self.eth_provider);

        EthRpcProvider { web_provider: self.web_provider, eth_provider }
    }

    pub async fn send_add_remove_liquidity_tx(
        &self,
        tx_req: TransactionRequestWithLiquidityMeta
    ) -> eyre::Result<TxHash> {
        Ok(self
            .eth_provider
            .send_transaction(tx_req.tx_request)
            .await?
            .watch()
            .await?)
    }
}

impl<P> AngstromDataApi for EthRpcProvider<P>
where
    P: Provider + Clone + 'static
{
    // async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
    //     let config_store = pool_config_store(self.provider()).await?;
    //     let partial_key_entries = config_store.all_entries();

    //     if partial_key_entries.is_empty() {
    //         return Ok(Vec::new())
    //     }

    //     println!("{partial_key_entries:?}");

    //     let mut multicall =
    // Multicall::with_provider_chain_id(self.provider().clone()).await?;
    //     multicall.set_version(3);

    //     let funcs = ControllerV1::abi::functions();
    //     let pools_fn = funcs.get("pools").unwrap().first().unwrap();

    //     partial_key_entries.iter().for_each(|partial_key| {
    //         let mut key = vec![0, 0, 0, 0, 0];
    //         key.extend(FixedBytes::from(*partial_key.pool_partial_key).0);

    //         // let other_key = FixedBytes::from(*partial_key.pool_partial_key);
    //         // let this = other_key.;

    //         multicall.add_call(
    //             CONTROLLER_V1_ADDRESS,
    //             pools_fn,
    //             &[DynSolValue::FixedBytes(B256::from_slice(&key), 27)],
    //             // &[],
    //             true
    //         );
    //     });

    //     let all_pools_call = multicall
    //         .as_aggregate_3()
    //         .call()
    //         .await?
    //         .returnData
    //         .into_iter()
    //         .map(|res| {
    //             if res.success {
    //                 Ok(poolsCall::abi_decode_returns(&*res.returnData,
    // true).unwrap())             } else {
    //                 Err(eyre::eyre!("{:?}", res.returnData))
    //             }
    //             // println!("{res:?}");
    //             // Ok::<_, eyre::ErrReport>(
    //             //     res.map(|val| {
    //             //         val.as_custom_struct()
    //             //             .unwrap()
    //             //             .2
    //             //             .iter()
    //             //             .map(|tuple_val| tuple_val.as_address().unwrap())
    //             //             .collect::<Vec<_>>()
    //             //     })
    //             //     .unwrap()
    //             // )
    //         })
    //         .collect::<Result<Vec<_>, _>>()
    //         .map_err(|e| eyre::eyre!(e))?;

    //     Ok(all_pools_call
    //         .into_iter()
    //         .map(|val| TokenPairInfo {
    //             // token0:    *val.first().unwrap(),
    //             // token1:    *val.last().unwrap(),
    //             token0:    val.asset0,
    //             token1:    val.asset1,
    //             is_active: false
    //         })
    //         .collect())
    // }

    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        let config_store = pool_config_store(self.provider()).await?;
        let partial_key_entries = config_store.all_entries();

        let all_pools_call = futures::future::join_all(partial_key_entries.iter().map(|key| {
            self.view_call(
                CONTROLLER_V1_ADDRESS,
                ControllerV1::poolsCall { key: FixedBytes::from(*key.pool_partial_key) }
            )
        }))
        .await
        .into_iter()
        .collect::<Result<Vec<_>, _>>()?;

        Ok(all_pools_call
            .into_iter()
            .map(|val| TokenPairInfo {
                token0:    val.asset0,
                token1:    val.asset1,
                is_active: true
            })
            .collect())
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        let (token0, token1) = sort_tokens(token0, token1);

        let config_store = pool_config_store(self.provider()).await?;
        let pool_config_store = config_store.get_entry(token0, token1).ok_or(eyre::eyre!(
            "no config store entry for tokens {token0:?} -
    {token1:?}"
        ))?;

        Ok(PoolKey {
            currency0:   token0,
            currency1:   token1,
            fee:         U24::from(pool_config_store.fee_in_e6),
            tickSpacing: I24::unchecked_from(pool_config_store.tick_spacing),
            hooks:       ANGSTROM_ADDRESS
        })
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter
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
                    .get_block(bn.into(), BlockTransactionsKind::Full)
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
        block_number: Option<u64>
    ) -> eyre::Result<EnhancedUniswapPool<DataLoader<PoolId>, PoolId>> {
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

    async fn binance_price(&self, token_address: Address) -> eyre::Result<f64> {
        let (unfmt_token_name, unfmt_token_symbol) = tokio::try_join!(
            self.view_call(token_address, MintableMockERC20::nameCall {}),
            self.view_call(token_address, MintableMockERC20::symbolCall {})
        )?;

        // let token_instance = Box::leak(Box::new(MintableMockERC20Instance::new(
        //     token_address,
        //     self.eth_provider.clone()
        // )));

        // let (unfmt_token_name, unfmt_token_symbol) =
        //     tokio::try_join!(token_instance.name(), token_instance.symbol(),)?;

        println!("{unfmt_token_name:?} - {unfmt_token_symbol:?}");

        let token_symbol = if unfmt_token_symbol._0.starts_with("W")
            && unfmt_token_name._0.to_lowercase().contains("wrapped")
        {
            unfmt_token_symbol._0[1..].to_string()
        } else {
            unfmt_token_symbol._0
        };

        let binance_pair = format!("{token_symbol}USDT");

        println!("{binance_pair}");

        let response: Value = self
            .web_provider
            .get(format!("{BINANCE_REST_API_BASE_URL}/api/v3/ticker/price?symbol={binance_pair}"))
            .send()
            .await?
            .json()
            .await?;
        let response = Box::leak(Box::new(response));

        println!("{:?}", response);

        if let Some(price) = response.get("price") {
            price
                .as_str()
                .map(|v| v.parse().ok())
                .flatten()
                .ok_or(eyre::eyre!("could not convert price to f64 for {binance_pair}"))
        } else {
            let err_msg = response
                .get("msg")
                .ok_or(eyre::eyre!("could not fetch binance price for {binance_pair}"))?
                .as_str()
                .ok_or(eyre::eyre!("could not convert price to f64 for {binance_pair}"))?;
            Err(eyre::eyre!(err_msg))
        }
    }
}

#[cfg(test)]
mod tests {

    use alloy_primitives::address;

    use super::*;
    use crate::test_utils::spawn_ws_provider;

    #[tokio::test]
    async fn test_all_token_pairs() {
        let provider = spawn_ws_provider().await.unwrap();

        let all_pairs = provider.all_token_pairs().await.unwrap();

        assert!(!all_pairs.is_empty());
        let first = all_pairs.first().unwrap();

        assert_ne!(Address::ZERO, first.token0);
        assert_ne!(Address::ZERO, first.token1);
    }

    #[tokio::test]
    async fn test_pool_key() {
        let provider = spawn_ws_provider().await.unwrap();
        let token0 = address!("2260fac5e5542a773aa44fbcfedf7c193bc2c599");
        let token1 = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");

        let pool_key = provider.pool_key(token0, token1).await.unwrap();
        let expected_pool_key = PoolKey {
            currency0:   token0,
            currency1:   token1,
            fee:         U24::ZERO,
            tickSpacing: I24::unchecked_from(60),
            hooks:       ANGSTROM_ADDRESS
        };

        assert_eq!(pool_key, expected_pool_key);
    }

    #[tokio::test]
    async fn test_binance_price() {
        let provider = spawn_ws_provider().await.unwrap();
        let price = provider
            .binance_price(address!("3d85e7b30be9fd7a4bad709d6ed2d130579f9a2e"))
            .await
            .unwrap();
        println!("PRICE FOR WETH: {price}");

        let price = provider
            .binance_price(address!("45cb6df752760cc995fe9b05c61ce6bd8776b1e7"))
            .await
            .unwrap();

        println!("PRICE FOR WBTC: {price}");
    }
}
