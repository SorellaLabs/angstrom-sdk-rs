use std::{
    collections::HashMap,
    sync::{Arc, RwLock}
};

use alloy_provider::{Provider, RootProvider};
use alloy_transport::BoxTransport;
use testing_tools::order_generator::OrderGenerator;
use tokio::sync::Notify;
use uniswap_v4::uniswap::pool_manager::{SyncedUniswapPools, TickRangeToLoad};

use crate::{
    apis::data_api::AngstromDataApi,
    providers::{AngstromProvider, EthRpcProvider},
    AngstromApi
};

pub async fn spawn_ws_provider() -> eyre::Result<EthRpcProvider<RootProvider>> {
    dotenv::dotenv().ok();
    let ws_url = std::env::var("ETH_WS_URL").expect("ETH_WS_URL not found in .env");

    EthRpcProvider::new(&ws_url).await
}

pub async fn spawn_angstrom_provider() -> eyre::Result<AngstromProvider> {
    dotenv::dotenv().ok();
    let http_url = std::env::var("ANGSTROM_HTTP_URL").expect("ANGSTROM_HTTP_URL not found in .env");

    AngstromProvider::new(http_url)
}

pub async fn spawn_angstrom_api() -> eyre::Result<AngstromApi<RootProvider>> {
    Ok(AngstromApi::new(spawn_ws_provider().await?, spawn_angstrom_provider().await?))
}

pub async fn make_generator<P>(
    provider: &EthRpcProvider<P>
) -> eyre::Result<(OrderGenerator, tokio::sync::mpsc::Receiver<(TickRangeToLoad, Arc<Notify>)>)>
where
    P: Provider + Clone
{
    let block_number = provider.eth_provider().get_block_number().await?;
    let pairs = provider.all_token_pairs().await?;

    let uniswap_pools = futures::future::join_all(pairs.into_iter().map(|pair| async move {
        let pool = provider
            .pool_data(pair.token0, pair.token1, Some(block_number))
            .await?;
        Ok::<_, eyre::ErrReport>((pool.address(), Arc::new(RwLock::new(pool))))
    }))
    .await
    .into_iter()
    .collect::<Result<HashMap<_, _>, _>>()?;

    let (tx, rx) = tokio::sync::mpsc::channel(100);
    let generator = OrderGenerator::new(
        SyncedUniswapPools::new(Arc::new(uniswap_pools), tx),
        block_number,
        20..50,
        0.5..0.7
    );

    Ok((generator, rx))
}
