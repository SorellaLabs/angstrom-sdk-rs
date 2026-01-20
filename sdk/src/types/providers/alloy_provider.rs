use std::{fmt::Debug, marker::PhantomData, ops::Deref};

use alloy_eips::BlockId;
use alloy_json_rpc::RpcError;
use alloy_network::{Ethereum, Network};
use alloy_primitives::{Address, TxKind};
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_sol_types::{SolCall, SolType};
use alloy_transport::TransportErrorKind;

/// Wrapper for alloy providers that implements SDK traits.
/// This wrapper is necessary to avoid trait coherence conflicts with
/// `RethDbProviderWrapper`.
#[derive(Clone)]
pub struct AlloyProviderWrapper<P, N: Network = Ethereum> {
    provider: P,
    _network: PhantomData<N>
}

impl<P: Debug, N: Network> Debug for AlloyProviderWrapper<P, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AlloyProviderWrapper")
            .field("provider", &self.provider)
            .finish()
    }
}

impl<P, N: Network> AlloyProviderWrapper<P, N> {
    pub fn new(provider: P) -> Self {
        Self { provider, _network: PhantomData }
    }

    pub fn provider(&self) -> &P {
        &self.provider
    }

    pub fn into_inner(self) -> P {
        self.provider
    }
}

impl<P, N: Network> Deref for AlloyProviderWrapper<P, N> {
    type Target = P;

    fn deref(&self) -> &Self::Target {
        &self.provider
    }
}

impl<P: Provider<N>, N: Network> Provider<N> for AlloyProviderWrapper<P, N> {
    fn root(&self) -> &RootProvider<N> {
        self.provider.root()
    }
}

impl<P: Provider<N>, N: Network> From<P> for AlloyProviderWrapper<P, N> {
    fn from(value: P) -> Self {
        Self::new(value)
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
