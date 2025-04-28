pub mod filler_orders;
pub mod valid_orders;

use alloy_provider::RootProvider;
use angstrom_types::primitive::AngstromSigner;

use crate::{
    AngstromApi,
    providers::backend::{AlloyRpcProvider, AngstromProvider}
};

#[cfg(not(feature = "testnet-sepolia"))]
const ANGSTROM_HTTP_URL: &str = "ANGSTROM_HTTP_URL";
#[cfg(feature = "testnet-sepolia")]
const ANGSTROM_HTTP_URL: &str = "ANGSTROM_SEPOLIA_HTTP_URL";
#[cfg(not(feature = "testnet-sepolia"))]
const ETH_WS_URL: &str = "ETH_WS_URL";
#[cfg(feature = "testnet-sepolia")]
const ETH_WS_URL: &str = "ETH_SEPOLIA_WS_URL";

pub fn angstrom_http_url() -> String {
    dotenv::dotenv().ok();
    std::env::var(ANGSTROM_HTTP_URL)
        .unwrap_or_else(|_| panic!("{ANGSTROM_HTTP_URL} not found in .env"))
}

pub fn eth_ws_url() -> String {
    dotenv::dotenv().ok();
    std::env::var(ETH_WS_URL).unwrap_or_else(|_| panic!("{ETH_WS_URL} not found in .env"))
}

pub fn testing_private_key() -> AngstromSigner {
    dotenv::dotenv().ok();
    AngstromSigner::new(
        std::env::var("TESTING_PRIVATE_KEY")
            .expect("TESTING_PRIVATE_KEY not found in .env")
            .parse()
            .unwrap()
    )
}

async fn spawn_angstrom_provider() -> eyre::Result<AngstromProvider<AlloyRpcProvider<RootProvider>>>
{
    AngstromProvider::new(&eth_ws_url(), &angstrom_http_url()).await
}

pub async fn spawn_angstrom_api() -> eyre::Result<AngstromApi<AlloyRpcProvider<RootProvider>>> {
    Ok(AngstromApi::new_with_provider(spawn_angstrom_provider().await?))
}

pub trait OrderExecutor {
    async fn execute_with_all_orders(self, f: ()) -> bool
    where
        Self: Sized;
}
