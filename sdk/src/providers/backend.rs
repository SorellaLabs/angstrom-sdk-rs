use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::Signature;
use alloy_provider::{
    Identity, Provider, RootProvider,
    fillers::{FillProvider, JoinFill, WalletFiller}
};
use alloy_signer::{Signer, SignerSync};
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::{WsClient, WsClientBuilder};

use crate::apis::node_api::{AngstromNodeApi, AngstromOrderApiClient};

pub type AlloyWalletRpcProvider<P> =
    FillProvider<JoinFill<Identity, WalletFiller<EthereumWallet>>, P>;

#[derive(Debug, Clone)]
pub struct AngstromProvider<P, T>
where
    P: Provider,
    T: AngstromOrderApiClient
{
    eth_provider:      P,
    angstrom_provider: T
}

impl<P: Provider> AngstromProvider<P, HttpClient> {
    pub fn new_angstrom_http(eth_provider: P, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self { eth_provider, angstrom_provider: HttpClient::builder().build(angstrom_url)? })
    }
}

impl<P: Provider> AngstromProvider<P, WsClient> {
    pub async fn new_angstrom_ws(eth_provider: P, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            eth_provider,
            angstrom_provider: WsClientBuilder::new().build(angstrom_url).await?
        })
    }
}

impl<P: Provider, T: AngstromOrderApiClient> AngstromProvider<P, T> {
    pub fn new_with_providers(eth_provider: P, angstrom_provider: T) -> Self {
        Self { eth_provider, angstrom_provider }
    }

    pub fn eth_provider(&self) -> &P {
        &self.eth_provider
    }

    pub fn with_wallet<S>(self, signer: S) -> AngstromProvider<AlloyWalletRpcProvider<P>, T>
    where
        S: Signer + SignerSync + TxSigner<Signature> + Send + Sync + 'static
    {
        let eth_provider = alloy_provider::builder::<Ethereum>()
            .wallet(EthereumWallet::new(signer))
            .connect_provider(self.eth_provider);

        AngstromProvider { eth_provider, angstrom_provider: self.angstrom_provider }
    }
}

impl<P: Provider, T: AngstromOrderApiClient> AngstromNodeApi<T> for AngstromProvider<P, T> {
    fn angstrom_rpc_provider(&self) -> &T {
        &self.angstrom_provider
    }
}

impl<P: Provider, T: AngstromOrderApiClient> Provider for AngstromProvider<P, T> {
    fn root(&self) -> &RootProvider {
        self.eth_provider.root()
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::b256;

    use super::*;
    use crate::{apis::AngstromDataApi, test_utils::spawn_angstrom_api};

    #[tokio::test]
    async fn test_thing() {
        let provider = spawn_angstrom_api().await.unwrap();

        let bundle = provider
            .get_bundle_by_tx_hash(
                b256!("0x32716081b3461e4f4770e14d97565c003aecf647837d151a8380f6b9722e7faf"),
                true
            )
            .await
            .unwrap();

        println!("{bundle:?}");
    }
}
