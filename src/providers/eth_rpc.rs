use alloy_provider::RootProvider;
use alloy_rpc_client::ClientBuilder;
use alloy_transport::BoxTransport;

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
