mod order_gen;

use std::str::FromStr;

use alloy_signer_local::{LocalSigner, PrivateKeySigner};
use angstrom_sdk_rs::{
    AngstromApi,
    apis::node_api::AngstromNodeApi,
    types::{USDC, WETH},
};

use order_gen::ValidOrderGenerator;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    dotenv::dotenv().ok();
    let angstrom_http_url = std::env::var("ANGSTROM_SEPOLIA_HTTP_URL")
        .expect("ANGSTROM_SEPOLIA_HTTP_URL not found in .env");
    let eth_ws_url =
        std::env::var("ETH_SEPOLIA_WS_URL").expect("ETH_SEPOLIA_WS_URL not found in .env");
    let signer_pk =
        std::env::var("TESTING_PRIVATE_KEY").expect("TESTING_PRIVATE_KEY not found in .env");

    let signer = PrivateKeySigner::from_str(&signer_pk)?;
    println!("FROM: {:?}", signer.address());

    let angstrom_api = AngstromApi::new(&eth_ws_url, &angstrom_http_url)
        .await?
        .with_all_fillers(signer);

    let order_generator = ValidOrderGenerator::new(angstrom_api.clone());

    let tob_order = order_generator.generate_valid_tob_order(USDC, WETH).await?;
    let tob_order_hash = angstrom_api.send_order(tob_order).await?;
    println!("TOB Order Hash: {tob_order_hash:?}");

    Ok(())
}
