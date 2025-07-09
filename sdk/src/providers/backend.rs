use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::{Address, Signature, TxHash, U256};
use alloy_provider::{
    Identity, Provider,
    fillers::{FillProvider, JoinFill, WalletFiller}
};
use alloy_signer::{Signer, SignerSync};
use angstrom_types::{
    contract_bindings::pool_manager::PoolManager::{self, PoolKey},
    contract_payloads::angstrom::{
        AngstromBundle, AngstromPoolConfigStore, AngstromPoolPartialKey
    },
    primitive::PoolId
};
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::{WsClient, WsClientBuilder};
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    apis::{
        AngstromUserApi,
        data_api::AngstromDataApi,
        node_api::{AngstromNodeApi, AngstromOrderApiClient}
    },
    types::{
        positions::{
            UserLiquidityPosition,
            fees::LiquidityPositionFees,
            utils::{UnpackedPositionInfo, UnpackedSlot0}
        },
        *
    }
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
    ) -> eyre::Result<TokenPair> {
        self.eth_provider
            .tokens_by_partial_pool_key(pool_partial_key, block_number)
            .await
    }

    async fn all_token_pairs_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<TokenPair>> {
        self.eth_provider
            .all_token_pairs_with_config_store(config_store, block_number)
            .await
    }

    async fn all_token_pairs(&self, block_number: Option<u64>) -> eyre::Result<Vec<TokenPair>> {
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
    ) -> eyre::Result<Vec<WithEthMeta<Vec<HistoricalOrders>>>> {
        self.eth_provider
            .historical_orders(filter, block_stream_buffer)
            .await
    }

    async fn historical_bundles(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<WithEthMeta<AngstromBundle>>> {
        self.eth_provider
            .historical_bundles(start_block, end_block, block_stream_buffer)
            .await
    }

    async fn historical_liquidity_changes(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>> {
        self.eth_provider
            .historical_liquidity_changes(start_block, end_block)
            .await
    }

    async fn historical_post_bundle_unlock_swaps(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::Swap>>> {
        self.eth_provider
            .historical_post_bundle_unlock_swaps(start_block, end_block)
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

    async fn slot0_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>
    ) -> eyre::Result<UnpackedSlot0> {
        self.eth_provider
            .slot0_by_pool_id(pool_id, block_number)
            .await
    }

    async fn get_bundle_by_block(
        &self,
        block_number: u64,
        verify_successful_tx: bool
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>> {
        self.eth_provider
            .get_bundle_by_block(block_number, verify_successful_tx)
            .await
    }

    async fn get_bundle_by_tx_hash(
        &self,
        tx_hash: TxHash,
        verify_successful_tx: bool
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>> {
        self.eth_provider
            .get_bundle_by_tx_hash(tx_hash, verify_successful_tx)
            .await
    }
}

#[async_trait::async_trait]
impl<P, T> AngstromUserApi for AngstromProvider<P, T>
where
    P: Provider,
    T: AngstromOrderApiClient
{
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)> {
        self.eth_provider
            .position_and_pool_info(position_token_id, block_number)
            .await
    }

    async fn position_liquidity(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<u128> {
        self.eth_provider
            .position_liquidity(position_token_id, block_number)
            .await
    }

    async fn all_user_positions(
        &self,
        owner: Address,
        start_token_id: U256,
        last_token_id: U256,
        pool_id: Option<PoolId>,
        max_results: Option<usize>,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        self.eth_provider
            .all_user_positions(owner, start_token_id, last_token_id, max_results, block_number)
            .await
    }

    async fn user_position_fees(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<LiquidityPositionFees> {
        self.eth_provider
            .user_position_fees(position_token_id, block_number)
            .await
    }
}

impl<P: Provider, T: AngstromOrderApiClient> AngstromNodeApi<T> for AngstromProvider<P, T> {
    fn angstrom_rpc_provider(&self) -> &T {
        &self.angstrom_provider
    }
}
