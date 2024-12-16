use std::collections::HashMap;

use alloy_provider::{Provider, RootProvider};
use alloy_transport::{BoxTransport, Transport};
use jsonrpsee_http_client::HttpClient;
use testing_tools::order_generator::OrderGenerator;

use crate::providers::{AngstromProvider, EthRpcProvider};

pub async fn spawn_ws_provider(
) -> eyre::Result<EthRpcProvider<RootProvider<BoxTransport>, BoxTransport>> {
    dotenv::dotenv().ok();
    let ws_url = std::env::var("ETH_WS_URL").expect("ETH_WS_URL not found in .env");

    Ok(EthRpcProvider::new_ws(ws_url).await?)
}

pub async fn spawn_angstrom_provider() -> eyre::Result<AngstromProvider> {
    dotenv::dotenv().ok();
    let http_url = std::env::var("ANGSTROM_HTTP_URL").expect("ANGSTROM_HTTP_URL not found in .env");

    let client = HttpClient::builder().build(http_url).unwrap();

    Ok(AngstromProvider::new(client))
}

pub async fn make_generator<P, T>(provider: &EthRpcProvider<P, T>) -> eyre::Result<OrderGenerator>
where
    P: Provider<T>,
    T: Transport + Clone,
{
    let block_number = provider.provider().block_number().await?;
    let uniswap_pools =
        futures::future::join_all(provider.all_token_pairs().await?.into_iter().map(
            |pair| async move {
                let pool = provider
                    .pool_data(pair.token0, pair.token1, Some(block_number))
                    .await?;
                Ok::<_, eyre::ErrReport>((pool.address(), pool))
            },
        ))
        .await
        .into_iter()
        .collect::<Result<HashMap<_, _>, _>>()?;

    let mut generator =
        OrderGenerator::new(Arc::new(uniswap_pools), block_number, 20..50, 0.5..0.7);

    Ok(generator)
}
