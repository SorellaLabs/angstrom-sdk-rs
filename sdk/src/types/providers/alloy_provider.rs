use std::ops::Deref;

use alloy_eips::BlockId;
use alloy_json_rpc::RpcError;
use alloy_network::{Ethereum, Network};
use alloy_primitives::{Address, StorageKey, StorageValue, TxKind};
use alloy_provider::{DynProvider, Provider, RootProvider};
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_sol_types::{SolCall, SolType};
use alloy_transport::TransportErrorKind;
use uniswap_storage::StorageSlotFetcher;

/// Wrapper for alloy providers that implements SDK traits.
/// This wrapper is necessary to avoid trait coherence conflicts with
/// `RethDbProviderWrapper`.
#[derive(Debug, Clone)]
pub struct AlloyProviderWrapper<N: Network = Ethereum> {
    provider: DynProvider<N>
}

impl<N: Network> AlloyProviderWrapper<N> {
    pub fn new(provider: impl Provider<N> + 'static) -> Self {
        Self { provider: DynProvider::new(provider) }
    }

    pub fn provider(&self) -> &DynProvider<N> {
        &self.provider
    }

    pub fn into_inner(self) -> DynProvider<N> {
        self.provider
    }
}

impl<N: Network> Deref for AlloyProviderWrapper<N> {
    type Target = DynProvider<N>;

    fn deref(&self) -> &Self::Target {
        &self.provider
    }
}

impl<N: Network> Provider<N> for AlloyProviderWrapper<N> {
    fn root(&self) -> &RootProvider<N> {
        self.provider.root()
    }
}

pub(crate) async fn alloy_view_call<P, IC>(
    provider: &P,
    block_number: Option<u64>,
    contract: Address,
    call: IC
) -> Result<Result<IC::Return, alloy_sol_types::Error>, RpcError<TransportErrorKind>>
where
    P: Provider + Clone,
    IC: SolCall + Send
{
    let tx = TransactionRequest {
        to: Some(TxKind::Call(contract)),
        input: TransactionInput::both(call.abi_encode().into()),
        ..Default::default()
    };

    let data = provider
        .call(tx)
        .block(block_number.map(Into::into).unwrap_or(BlockId::latest()))
        .await?;
    Ok(IC::abi_decode_returns(&data))
}

pub(crate) async fn alloy_view_deploy<P, N, IC>(
    provider: &P,
    block_number: Option<u64>,
    tx: <N as Network>::TransactionRequest
) -> Result<Result<IC::RustType, alloy_sol_types::Error>, RpcError<TransportErrorKind>>
where
    P: Provider<N>,
    N: Network,
    IC: SolType + Send
{
    let data = provider
        .call(tx)
        .block(block_number.map(Into::into).unwrap_or(BlockId::latest()))
        .await?;
    Ok(IC::abi_decode(&data))
}

#[async_trait::async_trait]
impl<N: Network> StorageSlotFetcher for AlloyProviderWrapper<N> {
    async fn storage_at(
        &self,
        address: Address,
        key: StorageKey,
        block_id: Option<BlockId>
    ) -> eyre::Result<StorageValue> {
        Ok(self
            .root()
            .get_storage_at(address, key.into())
            .block_id(block_id.unwrap_or(BlockId::latest()))
            .await?)
    }
}
