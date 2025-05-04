use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::{Address, Signature};
use alloy_provider::{
    Identity, Provider,
    fillers::{
        BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
    },
};
use alloy_signer::{Signer, SignerSync};
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey,
    contract_payloads::angstrom::{AngstromBundle, AngstromPoolConfigStore},
};
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::{WsClient, WsClientBuilder};
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    apis::{
        data_api::AngstromDataApi,
        node_api::{AngstromNodeApi, AngstromOrderApiClient},
        user_api::AngstromUserApi,
    },
    types::*,
};

pub type AlloyRpcProvider<P> = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    P,
>;

pub type AlloyWalletRpcProvider<P> =
    FillProvider<JoinFill<Identity, WalletFiller<EthereumWallet>>, P>;

#[derive(Debug, Clone)]
pub struct AngstromProvider<P, T>
where
    P: Provider,
    T: AngstromOrderApiClient,
{
    eth_provider: P,
    angstrom_provider: T,
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
            angstrom_provider: WsClientBuilder::new().build(angstrom_url).await?,
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

    pub(crate) fn with_wallet<S>(self, signer: S) -> AngstromProvider<AlloyWalletRpcProvider<P>, T>
    where
        S: Signer + SignerSync + TxSigner<Signature> + Send + Sync + 'static,
    {
        let eth_provider = alloy_provider::builder::<Ethereum>()
            .wallet(EthereumWallet::new(signer))
            .on_provider(self.eth_provider);

        AngstromProvider { eth_provider, angstrom_provider: self.angstrom_provider }
    }
}

impl<P, T> AngstromDataApi for AngstromProvider<P, T>
where
    P: Provider,
    T: AngstromOrderApiClient,
{
    async fn all_token_pairs(&self) -> eyre::Result<Vec<TokenPairInfo>> {
        self.eth_provider.all_token_pairs().await
    }

    async fn all_tokens(&self) -> eyre::Result<Vec<TokenInfoWithMeta>> {
        self.eth_provider.all_tokens().await
    }

    async fn pool_key(&self, token0: Address, token1: Address) -> eyre::Result<PoolKey> {
        self.eth_provider.pool_key(token0, token1).await
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
        block_stream_buffer: Option<usize>,
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        self.eth_provider
            .historical_orders(filter, block_stream_buffer)
            .await
    }

    async fn historical_bundles(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        block_stream_buffer: Option<usize>,
    ) -> eyre::Result<Vec<AngstromBundle>> {
        self.eth_provider
            .historical_bundles(start_block, end_block, block_stream_buffer)
            .await
    }

    async fn pool_data(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>,
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        self.eth_provider
            .pool_data(token0, token1, block_number)
            .await
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>,
    ) -> eyre::Result<AngstromPoolConfigStore> {
        self.eth_provider.pool_config_store(block_number).await
    }
}

impl<P: Provider, T: AngstromOrderApiClient> AngstromUserApi for AngstromProvider<P, T> {
    async fn get_positions(
        &self,
        user_address: Address,
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        self.eth_provider.get_positions(user_address).await
    }
}

impl<P: Provider, T: AngstromOrderApiClient> AngstromNodeApi<T> for AngstromProvider<P, T> {
    fn angstrom_rpc_provider(&self) -> &T {
        &self.angstrom_provider
    }
}
