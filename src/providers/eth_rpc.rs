use crate::types::{PoolMetadata, TokenInfo};

use alloy_primitives::{Address, Bytes, TxHash, TxKind, B256, U256};
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_client::ClientBuilder;
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_sol_types::SolCall;
use alloy_transport::BoxTransport;
use angstrom_types::sol_bindings::{grouped_orders::AllOrders, testnet::MockERC20};
use tokio::try_join;

use crate::apis::data_api::AngstromDataApi;

use super::eth_provider::EthProvider;

#[derive(Debug, Clone)]
#[repr(transparent)]
pub struct EthRpcProvider(RootProvider<BoxTransport>);

impl EthRpcProvider {
    pub fn new_http(http_url: impl ToString) -> eyre::Result<Self> {
        let builder = ClientBuilder::default().http(http_url.to_string().parse()?);
        let provider = RootProvider::new(builder).boxed();
        Ok(Self(provider))
    }

    #[cfg(feature = "ws")]
    pub async fn new_ws(ws_url: impl ToString) -> eyre::Result<Self> {
        let builder = ClientBuilder::default()
            .ws(alloy_provider::WsConnect::new(ws_url.to_string()))
            .await?;
        let provider = RootProvider::new(builder).boxed();
        Ok(Self(provider))
    }

    #[cfg(feature = "ipc")]
    pub async fn new_ipc(ipc_path: impl ToString) -> eyre::Result<Self> {
        let builder = ClientBuilder::default()
            .ipc(alloy_provider::IpcConnect::new(ipc_path.to_string()))
            .await?;
        let provider = RootProvider::new(builder).boxed();
        Ok(Self(provider))
    }
}

impl EthProvider for EthRpcProvider {
    async fn get_storage_at(&self, address: Address, key: U256) -> eyre::Result<U256> {
        Ok(self.0.get_storage_at(address, key).latest().await?)
    }

    async fn get_code_at(&self, address: Address) -> eyre::Result<Bytes> {
        Ok(self.0.get_code_at(address).latest().await?)
    }

    async fn view_call<IC>(&self, contract: Address, call: IC) -> eyre::Result<IC::Return>
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

    async fn get_erc20_info(&self, token_address: Address) -> eyre::Result<TokenInfo> {
        let symbol_fut = self.view_call(token_address, MockERC20::symbolCall {});
        let decimals_fut = self.view_call(token_address, MockERC20::decimalsCall {});
        let (symbols, decimals) = try_join!(symbol_fut, decimals_fut)?;

        Ok(TokenInfo {
            symbol: symbols._0,
            address: token_address,
            decimals: decimals._0,
        })
    }
}

impl AngstromDataApi for EthRpcProvider {
    async fn historical_trade(&self, tx_hash: TxHash, order_hash: B256) -> eyre::Result<AllOrders> {
        todo!()
    }

    async fn historical_trades(
        &self,
        from_block: Option<u64>,
        to_block: Option<u64>,
    ) -> eyre::Result<Vec<AllOrders>> {
        todo!()
    }
    async fn pool_metadata(&self, token0: Address, token1: Address) -> eyre::Result<PoolMetadata> {
        todo!()
    }
}
