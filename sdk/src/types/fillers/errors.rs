use alloy_json_rpc::RpcError;
use alloy_primitives::{Address, U256};
use alloy_transport::TransportErrorKind;

#[derive(Debug, thiserror::Error)]
pub enum FillerError {
    #[error("eth call error: {0:?}")]
    EthCall(#[from] RpcError<TransportErrorKind>),
    #[error("abi decoding error: {0:?}")]
    AbiDecodingError(#[from] alloy_sol_types::Error),
    #[error("signer error: {0:?}")]
    SignerError(#[from] alloy_signer::Error),
    #[error(
        "insufficient balance - token {0:?} with amount {1:?} in ourder, but user only has {2:?}"
    )]
    InsufficientBalanceError(Address, U256, U256)
}
