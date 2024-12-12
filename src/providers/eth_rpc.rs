use std::marker::PhantomData;

use super::eth_provider::EthProvider;
use crate::apis::data_api::AngstromDataApi;
use alloy_network::Ethereum;
use alloy_network::EthereumWallet;
use alloy_network::TxSigner;
use alloy_primitives::{Address, Bytes, TxKind, U256};
use alloy_provider::fillers::FillProvider;
use alloy_provider::fillers::JoinFill;
use alloy_provider::fillers::WalletFiller;
use alloy_provider::Identity;
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_client::ClientBuilder;
use alloy_rpc_types::{Block, BlockTransactionsKind, TransactionInput, TransactionRequest};
use alloy_signer::Signature;
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

impl<P, T> EthProvider for EthRpcProvider<P, T>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone,
{
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

    async fn current_block_number(&self) -> eyre::Result<u64> {
        Ok(self.0.get_block_number().await?)
    }

    async fn get_block(&self, number: u64, kind: BlockTransactionsKind) -> eyre::Result<Block> {
        Ok(self
            .0
            .get_block(number.into(), kind)
            .await?
            .ok_or(eyre::eyre!("no block found for block number {number}"))?)
    }

    async fn get_nonce(&self, address: Address) -> eyre::Result<u64> {
        Ok(self.0.get_transaction_count(address).await?)
    }
}

impl<P, T> AngstromDataApi for EthRpcProvider<P, T>
where
    P: Provider<T> + Clone + 'static,
    T: Transport + Clone,
{
}
