pub mod valid_test_params;

use alloy_primitives::{Address, address};
use alloy_provider::{
    Identity, Provider, RootProvider, WsConnect,
    fillers::{BlobGasFiller, ChainIdFiller, FillProvider, GasFiller, JoinFill, NonceFiller}
};
use alloy_signer_local::PrivateKeySigner;
use angstrom_types_primitives::primitive::{AngstromSigner, try_init_with_chain_id};
use auto_impl::auto_impl;
use jsonrpsee_http_client::HttpClient;
use op_alloy_network::Optimism;

use crate::l1::{AngstromApi, apis::AngstromOrderApiClient, providers::backend::AngstromProvider};

pub type AlloyRpcProvider<P> = FillProvider<
    JoinFill<
        Identity,
        JoinFill<GasFiller, JoinFill<BlobGasFiller, JoinFill<NonceFiller, ChainIdFiller>>>
    >,
    P
>;

pub const BASE_USDC: Address = address!("0xa0b86991c6218b36c1d19d4a2e9eb0ce3606eb48");
pub const BASE_WETH: Address = address!("0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2");

pub fn eth_ws_url() -> String {
    dotenv::dotenv().ok();
    std::env::var("BASE_ETH_WS_URL").unwrap_or_else(|_| panic!("BASE_ETH_WS_URL not found in .env"))
}

pub fn testing_private_key() -> AngstromSigner<PrivateKeySigner> {
    dotenv::dotenv().ok();
    AngstromSigner::new(
        std::env::var("TESTING_PRIVATE_KEY")
            .expect("TESTING_PRIVATE_KEY not found in .env")
            .parse()
            .unwrap()
    )
}

async fn eth_provider() -> eyre::Result<RootProvider<Optimism>> {
    Ok(RootProvider::builder()
        .connect_ws(WsConnect::new(eth_ws_url()))
        .await?)
}

pub trait OrderExecutor {
    async fn execute_with_all_orders(self, f: ()) -> bool
    where
        Self: Sized;
}
