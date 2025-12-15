pub mod filler_orders;
pub mod valid_test_params;

use alloy_primitives::{Address, address};
use alloy_provider::{
    Identity, Provider, RootProvider, WsConnect,
    fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller}
};
#[cfg(feature = "example-utils")]
use alloy_signer_local::PrivateKeySigner;
#[cfg(feature = "example-utils")]
use angstrom_types_primitives::primitive::AngstromSigner;
use angstrom_types_primitives::primitive::try_init_with_chain_id;
use jsonrpsee_http_client::HttpClient;

use crate::l1::{AngstromApi, providers::backend::AngstromProvider};

pub type AlloyRpcProvider<P> = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>
    >,
    P
>;

pub const USDC: Address = address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
pub const WETH: Address = address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");

#[cfg(feature = "example-utils")]
#[auto_impl::auto_impl(&, Box, Arc)]
pub trait AngstromOrderApiClientClone:
    crate::l1::apis::AngstromOrderApiClient + Clone + Sync
{
}
#[cfg(feature = "example-utils")]
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

#[cfg(feature = "example-utils")]
pub fn testing_private_key() -> AngstromSigner<PrivateKeySigner> {
    dotenv::dotenv().ok();
    AngstromSigner::new(
        std::env::var("TESTING_PRIVATE_KEY")
            .expect("TESTING_PRIVATE_KEY not found in .env")
            .parse()
            .unwrap()
    )
}

async fn spawn_angstrom_provider()
-> eyre::Result<AngstromProvider<AlloyRpcProvider<RootProvider>, HttpClient>> {
    let eth_provider = RootProvider::builder()
        .with_recommended_fillers()
        .connect_ws(WsConnect::new(eth_ws_url()))
        .await?;
    AngstromProvider::new_angstrom_http(eth_provider, &angstrom_http_url())
}

pub async fn spawn_angstrom_api()
-> eyre::Result<AngstromApi<AlloyRpcProvider<RootProvider>, HttpClient>> {
    let _ = try_init_with_chain_id(1);
    Ok(AngstromApi::new_with_provider(spawn_angstrom_provider().await?))
}
