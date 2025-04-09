use std::{
    collections::HashMap,
    fmt::Debug,
    sync::{Arc, RwLock},
};

use alloy_provider::{Provider, RootProvider};
use alloy_transport::BoxTransport;
use angstrom_types::sol_bindings::{
    grouped_orders::{AllOrders, FlashVariants, GroupedVanillaOrder, StandingVariants},
    rpc_orders::{
        ExactFlashOrder, ExactStandingOrder, PartialFlashOrder, PartialStandingOrder,
        TopOfBlockOrder,
    },
};
use testing_tools::order_generator::OrderGenerator;
use tokio::sync::Notify;
use uniswap_v4::uniswap::pool_manager::{SyncedUniswapPools, TickRangeToLoad};

use crate::{
    AngstromApi, AngstromFiller,
    apis::data_api::AngstromDataApi,
    providers::AngstromProvider,
    types::{ANGSTROM_HTTP_URL, ETH_WS_URL},
};

async fn spawn_angstrom_provider() -> eyre::Result<AngstromProvider<RootProvider>> {
    dotenv::dotenv().ok();
    let angstrom_http_url =
        std::env::var(ANGSTROM_HTTP_URL).expect(&format!("{ANGSTROM_HTTP_URL} not found in .env"));

    let eth_ws_url = std::env::var(ETH_WS_URL).expect(&format!("{ETH_WS_URL} not found in .env"));
    AngstromProvider::new(&eth_ws_url, &angstrom_http_url).await
}

pub async fn spawn_angstrom_api() -> eyre::Result<AngstromApi<RootProvider>> {
    Ok(AngstromApi::new(spawn_angstrom_provider().await?))
}

pub async fn make_order_generator<P>(
    provider: &AngstromProvider<P>,
) -> eyre::Result<(OrderGenerator, tokio::sync::mpsc::Receiver<(TickRangeToLoad, Arc<Notify>)>)>
where
    P: Provider + Clone,
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
        SyncedUniswapPools::new(Arc::new(uniswap_pools.into_iter().collect()), tx),
        block_number,
        20..50,
        0.5..0.7,
    );

    Ok((generator, rx))
}

pub fn generate_order_for_all(order_generator: &OrderGenerator) -> AllOrdersSpecific {
    let mut bitmap = 0x0000;
    let mut all_orders = Vec::new();

    loop {
        for order in order_generator.generate_orders() {
            if all_orders.is_empty() {
                all_orders.push(AllOrders::TOB(order.tob));
            }

            for book_order in &order.book {
                match book_order {
                    GroupedVanillaOrder::KillOrFill(FlashVariants::Partial(partial_flash)) => {
                        // check if bitmap contains a 1 in the first digit
                        if bitmap & 0b0001 != 0 {
                            all_orders.push(AllOrders::Flash(FlashVariants::Partial(
                                partial_flash.clone(),
                            )));
                            bitmap &= !0b0001;
                        }
                    }
                    GroupedVanillaOrder::KillOrFill(FlashVariants::Exact(exact_flash)) => {
                        // check if bitmap contains a 1 in the second digit
                        if bitmap & 0b0010 != 0 {
                            all_orders
                                .push(AllOrders::Flash(FlashVariants::Exact(exact_flash.clone())));
                            bitmap &= !0b0010;
                        }
                    }
                    GroupedVanillaOrder::Standing(StandingVariants::Partial(partial_standing)) => {
                        // check if bitmap contains a 1 in the third digit
                        if bitmap & 0b0100 != 0 {
                            all_orders.push(AllOrders::Standing(StandingVariants::Partial(
                                partial_standing.clone(),
                            )));
                            bitmap &= !0b0100;
                        }
                    }
                    GroupedVanillaOrder::Standing(StandingVariants::Exact(exact_standing)) => {
                        // check if bitmap contains a 1 in the fourth digit
                        if bitmap & 0b1000 != 0 {
                            all_orders.push(AllOrders::Standing(StandingVariants::Exact(
                                exact_standing.clone(),
                            )));
                            bitmap &= !0b1000;
                        }
                    }
                }
            }
        }

        if bitmap == 0x1111 {
            break AllOrdersSpecific::new(all_orders);
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct AllOrdersSpecific {
    pub tob: TopOfBlockOrder,
    pub partial_flash: PartialFlashOrder,
    pub exact_flash: ExactFlashOrder,
    pub partial_standing: PartialStandingOrder,
    pub exact_standing: ExactStandingOrder,
}

impl AllOrdersSpecific {
    fn new(orders: Vec<AllOrders>) -> Self {
        let mut tob: Option<TopOfBlockOrder> = None;
        let mut partial_flash: Option<PartialFlashOrder> = None;
        let mut exact_flash: Option<ExactFlashOrder> = None;
        let mut partial_standing: Option<PartialStandingOrder> = None;
        let mut exact_standing: Option<ExactStandingOrder> = None;

        orders.into_iter().for_each(|order| match order {
            AllOrders::Flash(FlashVariants::Exact(order)) => exact_flash = Some(order),
            AllOrders::Flash(FlashVariants::Partial(order)) => partial_flash = Some(order),
            AllOrders::Standing(StandingVariants::Exact(order)) => exact_standing = Some(order),
            AllOrders::Standing(StandingVariants::Partial(order)) => partial_standing = Some(order),
            AllOrders::TOB(order) => tob = Some(order),
        });

        Self {
            tob: tob.unwrap(),
            partial_flash: partial_flash.unwrap(),
            exact_flash: exact_flash.unwrap(),
            partial_standing: partial_standing.unwrap(),
            exact_standing: exact_standing.unwrap(),
        }
    }

    pub async fn test_filler_order(self, f: impl AsyncFn(AllOrders) -> bool) {
        assert!(f(AllOrders::TOB(self.tob)).await, "tob failed");
        assert!(
            f(AllOrders::Flash(FlashVariants::Partial(self.partial_flash))).await,
            "partial_flash failed"
        );
        assert!(
            f(AllOrders::Flash(FlashVariants::Exact(self.exact_flash))).await,
            "exact_flash failed"
        );
        assert!(
            f(AllOrders::Standing(StandingVariants::Partial(self.partial_standing))).await,
            "partial_standing failed"
        );
        assert!(
            f(AllOrders::Standing(StandingVariants::Exact(self.exact_standing))).await,
            "exact_standing failed"
        );
    }
}

pub fn match_all_orders<O>(
    order0: &AllOrders,
    order1: &AllOrders,
    f: impl Fn(&AllOrders) -> Option<O>,
) -> Option<(O, O)> {
    f(order0).zip(f(order1))
}
