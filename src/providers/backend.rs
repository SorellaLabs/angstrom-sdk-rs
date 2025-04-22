use alloy_network::{Ethereum, EthereumWallet, TxSigner};
use alloy_primitives::{Address, Signature, TxHash};
use alloy_provider::{
    Identity, Provider, RootProvider,
    fillers::{
        BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller, WalletFiller,
    },
};
use alloy_signer::{Signer, SignerSync};

use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey,
    contract_payloads::angstrom::AngstromPoolConfigStore,
};

use jsonrpsee_http_client::HttpClient;
use uniswap_v4::uniswap::{pool::EnhancedUniswapPool, pool_data_loader::DataLoader};

use crate::{
    apis::{data_api::AngstromDataApi, node_api::AngstromNodeApi, user_api::AngstromUserApi},
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
pub struct AngstromProvider<P>
where
    P: Provider,
{
    eth_provider: P,
    angstrom_provider: HttpClient,
}

impl AngstromProvider<AlloyRpcProvider<RootProvider>> {
    pub async fn new(eth_url: &str, angstrom_url: &str) -> eyre::Result<Self> {
        let angstrom_provider = HttpClient::builder().build(angstrom_url)?;
        Ok(Self {
            eth_provider: RootProvider::builder()
                .with_recommended_fillers()
                .connect(eth_url)
                .await?,
            angstrom_provider,
        })
    }
}
impl<P: Provider> AngstromProvider<P> {
    pub fn new_with_provider(eth_provider: P, angstrom_url: &str) -> eyre::Result<Self> {
        let angstrom_provider = HttpClient::builder().build(angstrom_url)?;
        Ok(Self { eth_provider, angstrom_provider })
    }

    pub fn eth_provider(&self) -> &P {
        &self.eth_provider
    }

    pub async fn send_add_remove_liquidity_tx(
        &self,
        tx_req: TransactionRequestWithLiquidityMeta,
    ) -> eyre::Result<TxHash> {
        Ok(self
            .eth_provider
            .send_transaction(tx_req.tx_request)
            .await?
            .watch()
            .await?)
    }

    pub(crate) fn with_wallet<S>(self, signer: S) -> AngstromProvider<AlloyWalletRpcProvider<P>>
    where
        S: Signer + SignerSync + TxSigner<Signature> + Send + Sync + 'static,
    {
        let eth_provider = alloy_provider::builder::<Ethereum>()
            .wallet(EthereumWallet::new(signer))
            .on_provider(self.eth_provider);

        AngstromProvider { eth_provider, angstrom_provider: self.angstrom_provider }
    }
}

impl<P> AngstromDataApi for AngstromProvider<P>
where
    P: Provider,
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
    ) -> eyre::Result<Vec<HistoricalOrders>> {
        self.eth_provider.historical_orders(filter).await
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

impl<P: Provider> AngstromUserApi for AngstromProvider<P> {
    async fn get_positions(
        &self,
        user_address: Address,
    ) -> eyre::Result<Vec<UserLiquidityPosition>> {
        self.eth_provider.get_positions(user_address).await
    }
}

impl<P: Provider> AngstromNodeApi for AngstromProvider<P> {
    fn angstrom_rpc_provider(&self) -> HttpClient {
        self.angstrom_provider.clone()
    }
}

#[cfg(test)]
mod data_api_tests {

    use alloy_primitives::aliases::I24;
    use alloy_primitives::aliases::U24;

    use crate::test_utils::spawn_angstrom_api;

    use super::*;

    #[tokio::test]
    async fn test_all_token_pairs() {
        let provider = spawn_angstrom_api().await.unwrap();

        let all_pairs = provider.all_token_pairs().await.unwrap();
        assert!(!all_pairs.is_empty());

        let contains = all_pairs
            .into_iter()
            .any(|pair| USDC == pair.token0 && WETH == pair.token1);
        assert!(contains);
    }

    #[tokio::test]
    async fn test_all_tokens() {
        let provider = spawn_angstrom_api().await.unwrap();

        let pool_keys = provider.all_tokens().await.unwrap();
        assert!(!pool_keys.is_empty());

        let contains_usdc = pool_keys
            .iter()
            .any(|token| token.address == USDC && &token.symbol == "USDC");
        let contains_weth = pool_keys
            .iter()
            .any(|token| token.address == WETH && &token.symbol == "WETH");

        assert!(contains_usdc);
        assert!(contains_weth);
    }

    #[tokio::test]
    async fn test_pool_key() {
        let provider = spawn_angstrom_api().await.unwrap();
        let token0 = USDC;
        let token1 = WETH;

        let pool_key = provider.pool_key(token0, token1).await.unwrap();
        let expected_pool_key = PoolKey {
            currency0: token0,
            currency1: token1,
            fee: U24::ZERO,
            tickSpacing: I24::unchecked_from(30),
            hooks: ANGSTROM_ADDRESS,
        };

        assert_eq!(pool_key, expected_pool_key);
    }

    #[tokio::test]
    async fn test_historical_orders() {
        todo!()
    }

    #[tokio::test]
    async fn test_pool_data() {
        todo!()
    }
}

#[cfg(test)]
mod user_tests {
    // use alloy_primitives::address;

    // use super::*;
    // use crate::test_utils::spawn_angstrom_api;

    #[tokio::test]
    async fn test_get_positions() {
        // let angstrom_api = spawn_angstrom_api().await.unwrap();

        // let positions = angstrom_api
        //     .get_positions(address!("0x796fB50EAe1456A523F869f6135dd557eeaEE226"))
        //     .await
        //     .unwrap();

        // println!("{positions:?}");

        todo!()
    }

    #[tokio::test]
    async fn test_get_positions_in_pool() {
        todo!()
    }
}
