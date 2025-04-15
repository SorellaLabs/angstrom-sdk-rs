#[derive(Debug, thiserror::Error)]
pub enum AngstromSdkError {
    #[error("filler error: {0:?}")]
    Filler(#[from] super::fillers::errors::FillerError),
    #[error("jsonrpsee error: {0:?}")]
    Jsonrpsee(#[from] jsonrpsee_core::ClientError),
    #[error("angstrom-rpc error: {0:?}")]
    AngstromRpc(String),
}
