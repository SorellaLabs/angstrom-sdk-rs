use alloy::transports::TransportErrorKind;
use alloy_json_rpc::RpcError;
use alloy_primitives::{Address, TxKind};
use alloy_provider::Provider;
use alloy_rpc_types::{TransactionInput, TransactionRequest};
use alloy_sol_types::SolCall;

pub(crate) async fn view_call<P, IC>(
    provider: &P,
    contract: Address,
    call: IC
) -> Result<Result<IC::Return, alloy_sol_types::Error>, RpcError<TransportErrorKind>>
where
    P: Provider,
    IC: SolCall + Send
{
    let tx = TransactionRequest {
        to: Some(TxKind::Call(contract)),
        input: TransactionInput::both(call.abi_encode().into()),
        ..Default::default()
    };

    let data = provider.call(tx).await?;
    Ok(IC::abi_decode_returns(&data))
}
