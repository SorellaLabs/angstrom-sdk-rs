use crate::apis::data_api::AngstromDataApi;
use crate::apis::utils::pool_config_store;
use alloy_primitives::aliases::I24;
use alloy_primitives::aliases::U24;
use alloy_primitives::Address;
use alloy_primitives::FixedBytes;
use alloy_rpc_types::BlockTransactionsKind;
use angstrom_types::contract_bindings::angstrom::Angstrom::PoolKey;
use angstrom_types::contract_bindings::controller_v_1::ControllerV1;
use angstrom_types::primitive::PoolId;
use futures::StreamExt;
use std::marker::PhantomData;
use std::sync::Arc;
use uniswap_v4::uniswap::pool::EnhancedUniswapPool;
use uniswap_v4::uniswap::pool_data_loader::DataLoader;

use crate::types::*;

use alloy_network::Ethereum;
use alloy_network::EthereumWallet;
use alloy_network::TxSigner;
use alloy_primitives::Signature;
use alloy_primitives::TxKind;
use alloy_provider::fillers::FillProvider;
use alloy_provider::fillers::JoinFill;
use alloy_provider::fillers::WalletFiller;
use alloy_provider::Identity;
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_client::ClientBuilder;
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_signer::Signer;
use alloy_signer::SignerSync;
use alloy_sol_types::SolCall;
use alloy_transport::{BoxTransport, Transport};

pub(crate) type RpcWalletProvider<P, T> =
    FillProvider<JoinFill<Identity, WalletFiller<EthereumWallet>>, P, T, Ethereum>;

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct EthRpcProvider<P, T>(P, PhantomData<T>);

impl EthRpcProvider<RootProvider<BoxTransport>, BoxTransport> {
    pub fn new_http(http_url: impl ToString) -> eyre::Result<Self> {
        let builder = ClientBuilder::default().http(http_url.to_string().parse()?);
        let provider = RootProvider::new(builder).boxed();
        Ok(Self(provider, PhantomData))
    }

    #[cfg(feature = "ws")]
    pub async fn new_ws(ws_url: impl ToString) -> eyre::Result<Self> {
        let builder = ClientBuilder::default()
            .ws(alloy_provider::WsConnect::new(ws_url.to_string()))
            .await?;
        let provider = RootProvider::new(builder).boxed();
        Ok(Self(provider, PhantomData))
    }

    #[cfg(feature = "ipc")]
    pub async fn new_ipc(ipc_path: impl ToString) -> eyre::Result<Self> {
        let builder = ClientBuilder::default()
            .ipc(alloy_provider::IpcConnect::new(ipc_path.to_string()))
            .await?;
        let provider = RootProvider::new(builder).boxed();
        Ok(Self(provider, PhantomData))
    }
}

impl<P, T> EthRpcProvider<P, T>
where
    P: Provider<T>,
    T: Transport + Clone,
{
    pub fn new(provider: P) -> Self {
        Self(provider, PhantomData)
    }

    pub fn provider(&self) -> &P {
        &self.0
    }

    pub async fn view_call<IC>(&self, contract: Address, call: IC) -> eyre::Result<IC::Return>
    where
        IC: SolCall + Send,
    {
        let tx = TransactionRequest {
            to: Some(TxKind::Call(contract)),
            input: TransactionInput::both(call.abi_encode().into()),
            ..Default::default()
        };

        Ok(IC::abi_decode_returns(&self.0.call(&tx).await?, true)?)
    }

    pub(crate) fn with_wallet<S>(self, signer: S) -> EthRpcProvider<RpcWalletProvider<P, T>, T>
    where
        S: Signer + SignerSync + TxSigner<Signature> + Send + Sync + 'static,
    {
        let p = alloy_provider::builder::<Ethereum>()
            .wallet(EthereumWallet::new(signer))
            .on_provider(self.0);

        EthRpcProvider(p, self.1)
    }
}

impl<P, T> AngstromDataApi for EthRpcProvider<P, T>
where
    P: Provider<T> + Clone,
    T: Transport + Clone,
{
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        let partial_keys = pool_config_store(self)
            .await?
            .all_entries()
            .iter()
            .map(|val| FixedBytes::from(*val.pool_partial_key))
            .collect::<Vec<_>>();

        let all_pools_call = self
            .view_call(
                CONTROLLER_V1_ADDRESS,
                ControllerV1::getAllPoolsCall {
                    storeKeys: partial_keys,
                },
            )
            .await?;

        Ok(all_pools_call
            ._0
            .into_iter()
            .map(|val| TokenPairInfo {
                token0: val.asset0,
                token1: val.asset1,
                is_active: true,
            })
            .collect())
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        let config_store = pool_config_store(self).await?;
        let pool_config_store = config_store.get_entry(token0, token1).ok_or(eyre::eyre!(
            "no config store entry for tokens {token0:?} - {token1:?}"
        ))?;

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
        filter: &HistoricalOrdersFilter,
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        let pool_stores = &AngstromPoolTokenIndexToPair::new_with_tokens(self, filter).await?;

        let start_block = filter.from_block.unwrap_or(ANGSTROM_DEPLOYED_BLOCK);
        let end_block = if let Some(e) = filter.to_block {
            e
        } else {
            self.0.get_block_number().await?
        };

        let mut block_stream = futures::stream::iter(start_block..end_block)
            .map(|bn| async move {
                let block = self
                    .0
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
        block_number: Option<u64>,
    ) -> eyre::Result<EnhancedUniswapPool<DataLoader<PoolId>, PoolId>> {
        let pool_key = self.pool_key(token0, token1).await?;
        let pool_id: PoolId = pool_key.clone().into();

        let data_loader =
            DataLoader::new_with_registry(pool_id, vec![pool_key].into(), POOL_MANAGER_ADDRESS);

        let mut enhanced_uni_pool = EnhancedUniswapPool::new(data_loader, 200);

        if let Some(bn) = block_number {
            enhanced_uni_pool
                .initialize(Some(bn), Arc::new(self.0.clone()))
                .await?;
        }

        Ok(enhanced_uni_pool)
    }
}
