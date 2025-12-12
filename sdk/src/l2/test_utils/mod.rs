pub mod valid_test_params;

use alloy_primitives::{Address, address};

pub const BASE_USDC: Address = address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
pub const BASE_WETH: Address = address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");

pub fn base_eth_ws_url() -> String {
    dotenv::dotenv().ok();
    std::env::var("BASE_ETH_WS_URL").unwrap_or_else(|_| panic!("BASE_ETH_WS_URL not found in .env"))
}

#[cfg(not(feature = "local-reth"))]
pub async fn eth_provider() -> eyre::Result<RootProvider<Optimism>> {
    use alloy_provider::{Provider, RootProvider, WsConnect};
    use angstrom_types_primitives::try_init_with_chain_id;
    use eth_network_exts::{EthNetworkExt, base_mainnet::BaseMainnetExt};
    use op_alloy_network::Optimism;

    let _ = try_init_with_chain_id(BaseMainnetExt::<()>::CHAIN_ID);
    Ok(RootProvider::builder()
        .connect_ws(WsConnect::new(base_eth_ws_url()))
        .await?)
}
