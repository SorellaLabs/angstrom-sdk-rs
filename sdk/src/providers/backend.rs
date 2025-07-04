use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::{Address, Signature};
use alloy_provider::{
    Identity, Provider,
    fillers::{FillProvider, JoinFill, WalletFiller}
};
use alloy_signer::{Signer, SignerSync};
use angstrom_types::contract_payloads::angstrom::{
    AngstromBundle, AngstromPoolConfigStore, AngstromPoolPartialKey
};
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::{WsClient, WsClientBuilder};
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    apis::{
        data_api::AngstromDataApi,
        node_api::{AngstromNodeApi, AngstromOrderApiClient},
        user_api::AngstromUserApi
    },
    types::*
};

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

#[async_trait::async_trait]
impl<P, T> AngstromDataApi for AngstromProvider<P, T>
where
    P: Provider,
    T: AngstromOrderApiClient
{
    async fn tokens_by_partial_pool_key(
        &self,
        pool_partial_key: AngstromPoolPartialKey,
        block_number: Option<u64>
    ) -> eyre::Result<TokenPairInfo> {
        self.eth_provider
            .tokens_by_partial_pool_key(pool_partial_key, block_number)
            .await
    }

    async fn all_token_pairs_with_config_store(
        &self,
        block_number: Option<u64>,
        config_store: AngstromPoolConfigStore
    ) -> eyre::Result<Vec<TokenPairInfo>> {
        self.eth_provider
            .all_token_pairs_with_config_store(block_number, config_store)
            .await
    }

    async fn all_token_pairs(&self, block_number: Option<u64>) -> eyre::Result<Vec<TokenPairInfo>> {
        self.eth_provider.all_token_pairs(block_number).await
    }

    async fn all_tokens(&self, block_number: Option<u64>) -> eyre::Result<Vec<Address>> {
        self.eth_provider.all_tokens(block_number).await
    }

    async fn pool_key_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<PoolKeyWithAngstromFee> {
        self.eth_provider
            .pool_key_by_tokens(token0, token1, block_number)
            .await
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        self.eth_provider
            .historical_orders(filter, block_stream_buffer)
            .await
    }

    async fn historical_bundles(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<AngstromBundle>> {
        self.eth_provider
            .historical_bundles(start_block, end_block, block_stream_buffer)
            .await
    }

    async fn pool_data_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        self.eth_provider
            .pool_data_by_tokens(token0, token1, block_number)
            .await
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>
    ) -> eyre::Result<AngstromPoolConfigStore> {
        self.eth_provider.pool_config_store(block_number).await
    }
}

// #[async_trait::async_trait]
// impl<P: Provider, T: AngstromOrderApiClient> AngstromUserApi for
// AngstromProvider<P, T> {     async fn get_positions(
//         &self,
//         user_address: Address,
//         block_number: Option<u64>
//     ) -> eyre::Result<Vec<UserLiquidityPosition>> {
//         self.eth_provider
//             .get_positions(user_address, block_number)
//             .await
//     }
// }

impl<P: Provider, T: AngstromOrderApiClient> AngstromNodeApi<T> for AngstromProvider<P, T> {
    fn angstrom_rpc_provider(&self) -> &T {
        &self.angstrom_provider
    }
}
