mod utils;

use std::str::FromStr;

use alloy_primitives::{Address, I256, TxKind};
use alloy_provider::{Provider, RootProvider};
use alloy_rpc_types::TransactionRequest;
use alloy_signer_local::PrivateKeySigner;
use alloy_sol_types::SolCall;
use angstrom_sdk_rs::{
    AngstromApi,
    apis::{AngstromDataApi, AngstromOrderBuilder},
    types::{USDC, WETH}
};

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

    let (_, pool) = angstrom_api.pool_data(USDC, WETH, None).await?;

    let amount = I256::unchecked_from(100000000i64);
    let order = AngstromOrderBuilder::modify_liquidity(
        USDC,
        WETH,
        pool.fetch_lowest_tick(),
        pool.fetch_highest_tick(),
        pool.tick_spacing,
        amount
    );

    // no callback contract is deployed so this will currently fail
    let callback_contract = Address::default();
    let tx_request = TransactionRequest {
        from: Some(signer.address()),
        to: Some(TxKind::Call(callback_contract)),
        input: order.abi_encode().into(),
        ..Default::default()
    };

    let tx_hash = angstrom_api
        .eth_provider()
        .send_transaction(tx_request)
        .await?
        .watch()
        .await?;
    println!("Add Liquidity Tx Hash: {tx_hash:?}");

    Ok(())
}
