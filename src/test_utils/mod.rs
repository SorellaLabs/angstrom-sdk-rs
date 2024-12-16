use alloy_provider::RootProvider;
use alloy_transport::BoxTransport;

use crate::providers::EthRpcProvider;

pub async fn spawn_ws_provider(
) -> eyre::Result<EthRpcProvider<RootProvider<BoxTransport>, BoxTransport>> {
    let ws_url = "ws://35.245.117.24:8546";
    Ok(EthRpcProvider::new_ws(ws_url).await?)
}
