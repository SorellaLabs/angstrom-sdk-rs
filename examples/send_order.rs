mod utils;

use std::str::FromStr;

use alloy_provider::{Provider, RootProvider};
use alloy_signer_local::PrivateKeySigner;
use angstrom_sdk_rs::{
    AngstromApi,
    apis::AngstromNodeApi,
    types::{USDC, WETH},
};
use utils::order_gen::ValidOrderGenerator;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv::dotenv().ok();
    let angstrom_http_url = &std::env::var("ANGSTROM_SEPOLIA_HTTP_URL")
        .expect("ANGSTROM_SEPOLIA_HTTP_URL not found in .env");
    let eth_ws_url =
        &std::env::var("ETH_SEPOLIA_WS_URL").expect("ETH_SEPOLIA_WS_URL not found in .env");
    let signer_pk =
        &std::env::var("TESTING_PRIVATE_KEY").expect("TESTING_PRIVATE_KEY not found in .env");

    let signer = PrivateKeySigner::from_str(signer_pk)?;

    let eth_provider = RootProvider::builder()
        .with_recommended_fillers()
        .connect(&eth_ws_url)
        .await?;

    let angstrom_api = AngstromApi::new_angstrom_http(eth_provider, angstrom_http_url)?
        .with_all_fillers(signer.clone());
    let order_generator = ValidOrderGenerator::new(angstrom_api.clone());
    let tob_order = order_generator.generate_valid_tob_order(USDC, WETH).await?;

    let tob_order_hash = angstrom_api.send_order(tob_order).await?;
    println!("TOB Order Hash: {tob_order_hash:?}");

    Ok(())
}
