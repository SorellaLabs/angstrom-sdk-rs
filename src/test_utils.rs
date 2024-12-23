use std::collections::HashMap;

use crate::{
    apis::data_api::AngstromDataApi,
    providers::{AngstromProvider, EthRpcProvider},
};
use alloy_provider::{Provider, RootProvider};
use alloy_transport::{BoxTransport, Transport};
use std::sync::Arc;
use std::sync::RwLock;
use testing_tools::order_generator::OrderGenerator;

pub async fn spawn_ws_provider(
) -> eyre::Result<EthRpcProvider<RootProvider<BoxTransport>, BoxTransport>> {
    dotenv::dotenv().ok();
    let ws_url = std::env::var("ETH_WS_URL").expect("ETH_WS_URL not found in .env");

    Ok(EthRpcProvider::new_ws(ws_url).await?)
}

pub async fn spawn_angstrom_provider() -> eyre::Result<AngstromProvider> {
    dotenv::dotenv().ok();
    let http_url = std::env::var("ANGSTROM_HTTP_URL").expect("ANGSTROM_HTTP_URL not found in .env");

    Ok(AngstromProvider::new(http_url)?)
}

pub async fn make_generator<P, T>(provider: &EthRpcProvider<P, T>) -> eyre::Result<OrderGenerator>
where
    P: Provider<T> + Clone,
    T: Transport + Clone,
{
    let block_number = provider.provider().get_block_number().await?;
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

    let generator = OrderGenerator::new(Arc::new(uniswap_pools), block_number, 20..50, 0.5..0.7);

    Ok(generator)
}
