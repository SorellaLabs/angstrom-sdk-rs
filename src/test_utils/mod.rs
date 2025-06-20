pub mod filler_orders;
pub mod valid_orders;

use crate::apis::AngstromOrderApiClient;

use crate::{AngstromApi, providers::backend::AngstromProvider};
use alloy_provider::WsConnect;
use alloy_provider::{
    Identity, Provider, RootProvider,
    fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller},
};
use alloy_signer_local::PrivateKeySigner;
use angstrom_types::primitive::{AngstromSigner, init_with_chain_id};
use auto_impl::auto_impl;
use jsonrpsee_http_client::HttpClient;

pub const USDC: alloy_primitives::Address =
    alloy_primitives::address!("0x1c7d4b196cb0c7b01d743fbc6116a902379c7238");

pub const WETH: alloy_primitives::Address =
    alloy_primitives::address!("0xfff9976782d46cc05630d1f6ebab18b2324d6b14");

pub const UNI: alloy_primitives::Address =
    alloy_primitives::address!("0x1f9840a85d5af5bf1d1762f925bdaddc4201f984");

pub type AlloyRpcProvider<P> = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>,
    >,
    P,
>;

#[auto_impl(&, Box, Arc)]
pub trait AngstromOrderApiClientClone: AngstromOrderApiClient + Clone + Sync {}
impl AngstromOrderApiClientClone for HttpClient {}

pub fn angstrom_http_url() -> String {
    dotenv::dotenv().ok();
    std::env::var("ANGSTROM_HTTP_URL")
        .unwrap_or_else(|_| panic!("ANGSTROM_HTTP_URL not found in .env"))
}

pub fn eth_ws_url() -> String {
    dotenv::dotenv().ok();
    std::env::var("ETH_WS_URL").unwrap_or_else(|_| panic!("ETH_WS_URL not found in .env"))
}

pub fn testing_private_key() -> AngstromSigner<PrivateKeySigner> {
    dotenv::dotenv().ok();
    AngstromSigner::new(
        std::env::var("TESTING_PRIVATE_KEY")
            .expect("TESTING_PRIVATE_KEY not found in .env")
            .parse()
            .unwrap(),
    )
}

async fn spawn_angstrom_provider()
-> eyre::Result<AngstromProvider<AlloyRpcProvider<RootProvider>, HttpClient>> {
    let eth_provider = RootProvider::builder()
        .with_recommended_fillers()
        .connect_ws(WsConnect::new(eth_ws_url()))
        .await?;
    Ok(AngstromProvider::new_angstrom_http(eth_provider, &angstrom_http_url())?)
}

pub async fn spawn_angstrom_api()
-> eyre::Result<AngstromApi<AlloyRpcProvider<RootProvider>, HttpClient>> {
    init_with_chain_id(11155111);
    Ok(AngstromApi::new_with_provider(spawn_angstrom_provider().await?))
}

pub trait OrderExecutor {
    async fn execute_with_all_orders(self, f: ()) -> bool
    where
        Self: Sized;
}
