use alloy_eips::BlockId;
use alloy_json_rpc::RpcError;
use alloy_network::Network;
use alloy_primitives::{Address, TxKind};
use alloy_provider::Provider;
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_sol_types::{SolCall, SolType};
use alloy_transport::TransportErrorKind;

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
