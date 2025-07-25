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
