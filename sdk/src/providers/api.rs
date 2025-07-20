use alloy_network::TxSigner;
use alloy_primitives::{Address, FixedBytes, Signature, TxHash, U256};
use alloy_provider::Provider;
use alloy_signer::{Signer, SignerSync};
use angstrom_types::{
    contract_bindings::pool_manager::PoolManager::{self, PoolKey},
    contract_payloads::angstrom::{
        AngstromBundle, AngstromPoolConfigStore, AngstromPoolPartialKey
    },
    primitive::PoolId,
    sol_bindings::grouped_orders::AllOrders
};
use jsonrpsee_http_client::HttpClient;
use jsonrpsee_ws_client::WsClient;
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    apis::{
        AngstromUserApi,
        data_api::AngstromDataApi,
        node_api::{AngstromNodeApi, AngstromOrderApiClient}
    },
    providers::backend::{AlloyWalletRpcProvider, AngstromProvider},
    types::{
        HistoricalOrders, HistoricalOrdersFilter, PoolKeyWithAngstromFee, TokenPair, WithEthMeta,
        contracts::{UnpackedPositionInfo, UnpackedSlot0, UserLiquidityPosition},
        errors::AngstromSdkError,
        fees::LiquidityPositionFees,
        fillers::{
            AngstromFillProvider, AngstromFiller, AngstromSignerFiller, FillWrapper,
            NonceGeneratorFiller, TokenBalanceCheckFiller
        }
    }
};

#[derive(Clone)]
pub struct AngstromApi<P, T, F = ()>
where
    P: Provider,
    T: AngstromOrderApiClient
{
    provider: AngstromProvider<P, T>,
    filler:   F
}

impl<P: Provider> AngstromApi<P, HttpClient> {
    pub fn new_angstrom_http(eth_provider: P, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            provider: AngstromProvider::new_angstrom_http(eth_provider, angstrom_url)?,
            filler:   ()
        })
    }
}

impl<P: Provider> AngstromApi<P, WsClient> {
    pub async fn new_angstrom_ws(eth_provider: P, angstrom_url: &str) -> eyre::Result<Self> {
        Ok(Self {
            provider: AngstromProvider::new_angstrom_ws(eth_provider, angstrom_url).await?,
            filler:   ()
        })
    }
}

impl<P, T> AngstromApi<P, T>
where
    P: Provider,
    T: AngstromOrderApiClient
{
    #[allow(unused)]
    pub fn new_with_provider(provider: AngstromProvider<P, T>) -> Self {
        Self { provider, filler: () }
    }
}

impl<P, T, F> AngstromApi<P, T, F>
where
    P: Provider,
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    pub fn eth_provider(&self) -> &P {
        self.provider.eth_provider()
    }

    pub fn angstrom_rpc_provider(&self) -> &T {
        self.provider.angstrom_rpc_provider()
    }

    pub fn angstrom_provider(&self) -> &AngstromProvider<P, T> {
        &self.provider
    }

    pub fn with_filler<F1: AngstromFiller>(
        self,
        filler: F1
    ) -> AngstromApi<P, T, AngstromFillProvider<F, F1>> {
        AngstromApi { provider: self.provider, filler: self.filler.wrap_with_filler(filler) }
    }

    pub fn with_nonce_generator_filler(
        self
    ) -> AngstromApi<P, T, AngstromFillProvider<F, NonceGeneratorFiller>> {
        AngstromApi {
            provider: self.provider,
            filler:   self.filler.wrap_with_filler(NonceGeneratorFiller)
        }
    }

    pub fn with_token_balance_filler(
        self
    ) -> AngstromApi<P, T, AngstromFillProvider<F, TokenBalanceCheckFiller>> {
        AngstromApi {
            provider: self.provider,
            filler:   self.filler.wrap_with_filler(TokenBalanceCheckFiller)
        }
    }

    pub fn with_angstrom_signer_filler<S>(
        self,
        signer: S
    ) -> AngstromApi<AlloyWalletRpcProvider<P>, T, AngstromFillProvider<F, AngstromSignerFiller<S>>>
    where
        S: Signer + SignerSync + TxSigner<Signature> + Clone + Send + Sync + 'static,
        AngstromSignerFiller<S>: FillWrapper
    {
        AngstromApi {
            provider: self.provider.with_wallet(signer.clone()),
            filler:   self
                .filler
                .wrap_with_filler(AngstromSignerFiller::new(signer))
        }
    }

    pub fn with_all_fillers<S>(
        self,
        signer: S
    ) -> AngstromApi<
        P,
        T,
        AngstromFillProvider<
            AngstromFillProvider<
                AngstromFillProvider<F, NonceGeneratorFiller>,
                TokenBalanceCheckFiller
            >,
            AngstromSignerFiller<S>
        >
    >
    where
        S: Signer + SignerSync + Send + Clone,
        AngstromSignerFiller<S>: FillWrapper,
        P: Provider
    {
        AngstromApi {
            provider: self.provider,
            filler:   self
                .filler
                .wrap_with_filler(NonceGeneratorFiller)
                .wrap_with_filler(TokenBalanceCheckFiller)
                .wrap_with_filler(AngstromSignerFiller::new(signer))
        }
    }

    pub fn from_address(&self) -> Option<Address> {
        self.filler.from()
    }
}

#[async_trait::async_trait]
impl<P, T, F> AngstromNodeApi<T> for AngstromApi<P, T, F>
where
    P: Provider,
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    fn angstrom_rpc_provider(&self) -> &T {
        self.provider.angstrom_rpc_provider()
    }

    async fn send_order(&self, mut order: AllOrders) -> Result<FixedBytes<32>, AngstromSdkError> {
        self.filler.fill(&self.provider, &mut order).await?;

        self.provider.send_order(order).await
    }

    async fn send_orders(
        &self,
        mut orders: Vec<AllOrders>
    ) -> Result<Vec<Result<FixedBytes<32>, AngstromSdkError>>, AngstromSdkError> {
        self.filler.fill_many(&self.provider, &mut orders).await?;

        self.provider.send_orders(orders).await
    }
}

#[async_trait::async_trait]
impl<P, T, F> AngstromDataApi for AngstromApi<P, T, F>
where
    P: Provider,
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    async fn tokens_by_partial_pool_key(
        &self,
        pool_partial_key: AngstromPoolPartialKey,
        block_number: Option<u64>
    ) -> eyre::Result<TokenPair> {
        self.provider
            .tokens_by_partial_pool_key(pool_partial_key, block_number)
            .await
    }

    async fn all_token_pairs_with_config_store(
        &self,
        config_store: AngstromPoolConfigStore,
        block_number: Option<u64>
    ) -> eyre::Result<Vec<TokenPair>> {
        self.provider
            .all_token_pairs_with_config_store(config_store, block_number)
            .await
    }

    async fn all_token_pairs(&self, block_number: Option<u64>) -> eyre::Result<Vec<TokenPair>> {
        self.provider.all_token_pairs(block_number).await
    }

    async fn all_tokens(&self, block_number: Option<u64>) -> eyre::Result<Vec<Address>> {
        self.provider.all_tokens(block_number).await
    }

    async fn historical_orders(
        &self,
        filter: HistoricalOrdersFilter,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<WithEthMeta<Vec<HistoricalOrders>>>> {
        self.provider
            .historical_orders(filter, block_stream_buffer)
            .await
    }

    async fn pool_data_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<(u64, EnhancedUniswapPool<DataLoader>)> {
        self.provider
            .pool_data_by_tokens(token0, token1, block_number)
            .await
    }

    async fn historical_bundles(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>,
        block_stream_buffer: Option<usize>
    ) -> eyre::Result<Vec<WithEthMeta<AngstromBundle>>> {
        self.provider
            .historical_bundles(start_block, end_block, block_stream_buffer)
            .await
    }

    async fn historical_liquidity_changes(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::ModifyLiquidity>>> {
        self.provider
            .historical_liquidity_changes(start_block, end_block)
            .await
    }

    async fn historical_post_bundle_unlock_swaps(
        &self,
        start_block: Option<u64>,
        end_block: Option<u64>
    ) -> eyre::Result<Vec<WithEthMeta<PoolManager::Swap>>> {
        self.provider
            .historical_post_bundle_unlock_swaps(start_block, end_block)
            .await
    }

    async fn pool_key_by_tokens(
        &self,
        token0: Address,
        token1: Address,
        block_number: Option<u64>
    ) -> eyre::Result<PoolKeyWithAngstromFee> {
        self.provider
            .pool_key_by_tokens(token0, token1, block_number)
            .await
    }

    async fn pool_config_store(
        &self,
        block_number: Option<u64>
    ) -> eyre::Result<AngstromPoolConfigStore> {
        self.provider.pool_config_store(block_number).await
    }

    async fn slot0_by_pool_id(
        &self,
        pool_id: PoolId,
        block_number: Option<u64>
    ) -> eyre::Result<UnpackedSlot0> {
        self.provider.slot0_by_pool_id(pool_id, block_number).await
    }

    async fn get_bundle_by_block(
        &self,
        block_number: u64,
        verify_successful_tx: bool
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>> {
        self.provider
            .get_bundle_by_block(block_number, verify_successful_tx)
            .await
    }

    async fn get_bundle_by_tx_hash(
        &self,
        tx_hash: TxHash,
        verify_successful_tx: bool
    ) -> eyre::Result<Option<WithEthMeta<AngstromBundle>>> {
        self.provider
            .get_bundle_by_tx_hash(tx_hash, verify_successful_tx)
            .await
    }
}

#[async_trait::async_trait]
impl<P, T, F> AngstromUserApi for AngstromApi<P, T, F>
where
    P: Provider,
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    async fn position_and_pool_info(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<(PoolKey, UnpackedPositionInfo)> {
        self.provider
            .position_and_pool_info(position_token_id, block_number)
            .await
    }

    async fn position_liquidity(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<u128> {
        self.provider
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
        self.provider
            .all_user_positions(
                owner,
                start_token_id,
                last_token_id,
                pool_id,
                max_results,
                block_number
            )
            .await
    }

    async fn user_position_fees(
        &self,
        position_token_id: U256,
        block_number: Option<u64>
    ) -> eyre::Result<LiquidityPositionFees> {
        self.provider
            .user_position_fees(position_token_id, block_number)
            .await
    }
}

#[cfg(test)]
impl<P, T, F> AngstromApi<P, T, F>
where
    P: Provider,
    F: AngstromFiller,
    T: AngstromOrderApiClient
{
    pub(crate) async fn fill(
        &self,
        order: &mut AllOrders
    ) -> Result<(), crate::types::fillers::errors::FillerError> {
        self.filler.fill(&self.provider, order).await
    }
}
