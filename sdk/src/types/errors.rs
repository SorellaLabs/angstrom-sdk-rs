use alloy::transports::TransportErrorKind;
use alloy_json_rpc::RpcError;

#[derive(Debug, thiserror::Error)]
pub enum AngstromSdkError {
    #[error("eth call error: {0:?}")]
    EthCall(#[from] RpcError<TransportErrorKind>),
    #[error("filler error: {0:?}")]
    Filler(#[from] super::fillers::errors::FillerError),
    #[error("jsonrpsee error: {0:?}")]
    Jsonrpsee(#[from] jsonrpsee_core::ClientError),
    #[error("angstrom-rpc error: {0:?}")]
    AngstromRpc(String),
    #[error(transparent)]
    Deser(#[from] serde_json::Error),
}
