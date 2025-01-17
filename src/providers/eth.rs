use std::sync::Arc;

use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::{
    aliases::{I24, U24},
    Address, FixedBytes, PrimitiveSignature, TxHash, TxKind
};
use alloy_provider::{
    fillers::{FillProvider, JoinFill, WalletFiller},
    Identity, Provider, RootProvider
};
use alloy_rpc_types::{BlockTransactionsKind, TransactionInput, TransactionRequest};
use alloy_signer::{Signer, SignerSync};
use alloy_sol_types::SolCall;
use alloy_transport::BoxTransport;
use angstrom_types::{
    contract_bindings::{angstrom::Angstrom::PoolKey, controller_v_1::ControllerV1},
    primitive::PoolId
};
use futures::StreamExt;
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    apis::{data_api::AngstromDataApi, utils::pool_config_store},
    types::*
};

pub(crate) type RpcWalletProvider<P> =
    FillProvider<JoinFill<Identity, WalletFiller<EthereumWallet>>, P, BoxTransport, Ethereum>;

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct EthRpcProvider<P>(P)
where
    P: Provider + Clone;

impl EthRpcProvider<RootProvider<BoxTransport>> {
    /// based on the url passed in, will auto parse to http,ws or ipc
    pub async fn new(url: &str) -> eyre::Result<Self> {
        Ok(Self(
            RootProvider::<BoxTransport, _>::builder()
                .on_builtin(url)
                .await?
        ))
    }
}
impl<P: Provider + Clone> EthRpcProvider<P> {
    pub fn provider(&self) -> &P {
        &self.0
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

        Ok(IC::abi_decode_returns(&self.provider().call(&tx).await?, true)?)
    }

    pub(crate) fn with_wallet<S>(self, signer: S) -> EthRpcProvider<RpcWalletProvider<P>>
    where
        S: Signer + SignerSync + TxSigner<PrimitiveSignature> + Send + Sync + 'static
    {
        let p = alloy_provider::builder::<Ethereum>()
            .wallet(EthereumWallet::new(signer))
            .on_provider(self.0);

        EthRpcProvider(p)
    }

    pub async fn send_add_remove_liquidity_tx(
        &self,
        tx_req: TransactionRequestWithLiquidityMeta
    ) -> eyre::Result<TxHash> {
        Ok(self
            .0
            .send_transaction(tx_req.tx_request)
            .await?
            .watch()
            .await?)
    }
}

impl<P> AngstromDataApi for EthRpcProvider<P>
where
    P: Provider + Clone
{
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        let partial_keys = pool_config_store(self.provider())
            .await?
            .all_entries()
            .iter()
            .map(|val| FixedBytes::from(*val.pool_partial_key))
            .collect::<Vec<_>>();

        let all_pools_call = self
            .view_call(
                CONTROLLER_V1_ADDRESS,
                ControllerV1::getAllPoolsCall { storeKeys: partial_keys }
            )
            .await?;

        Ok(all_pools_call
            ._0
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
        let pool_config_store = config_store
            .get_entry(token0, token1)
            .ok_or(eyre::eyre!("no config store entry for tokens {token0:?} - {token1:?}"))?;

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
        let end_block =
            if let Some(e) = filter.to_block { e } else { self.0.get_block_number().await? };

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
        block_number: Option<u64>
    ) -> eyre::Result<EnhancedUniswapPool<DataLoader<PoolId>, PoolId>> {
        let (token0, token1) = sort_tokens(token0, token1);

        let pool_key = self.pool_key(token0, token1).await?;
        let pool_id: PoolId = pool_key.clone().into();

        let data_loader =
            DataLoader::new_with_registry(pool_id, vec![pool_key].into(), POOL_MANAGER_ADDRESS);

        let mut enhanced_uni_pool = EnhancedUniswapPool::new(data_loader, 200);

        let block_number =
            if let Some(bn) = block_number { bn } else { self.0.get_block_number().await? };

        enhanced_uni_pool
            .initialize(Some(block_number), Arc::new(self.0.clone()))
            .await?;

        Ok(enhanced_uni_pool)
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
        let token0 = address!("c02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");
        let token1 = address!("cbcb9b1dff95bc829c17290c6c096c105974a14d");

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
}
