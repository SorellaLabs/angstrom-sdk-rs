use crate::apis::node_api::AngstromNodeApi;
use crate::providers::{AngstromProvider, EthProvider};
use crate::types::TransactionRequestWithLiquidityMeta;
use alloy_primitives::Address;
use angstrom_types::sol_bindings::grouped_orders::AllOrders;
use angstrom_types::sol_bindings::grouped_orders::StandingVariants;

use super::{AngstromFiller, FillFrom, FillerOrder};

#[derive(Clone, Copy, Debug, Default)]
pub struct NonceGeneratorFiller {
    my_address: Address,
}

impl NonceGeneratorFiller {
    pub fn new(my_address: Address) -> Self {
        Self { my_address }
    }
}

impl AngstromFiller for NonceGeneratorFiller {
    type FillOutput = Option<u64>;

    async fn prepare<E: EthProvider>(
        &self,
        eth_provider: &E,
        angstrom_provider: &AngstromProvider,
        order: &FillerOrder,
    ) -> eyre::Result<Self::FillOutput> {
        if !order_contains_nonce(order) {
            return Ok(None);
        }

        let current_nonce = eth_provider.get_nonce(self.my_address).await?;
        let pending_orders = angstrom_provider
            .pending_orders(vec![self.my_address])
            .await?
            .len() as u64;

        Ok(Some(current_nonce + pending_orders + 1))
    }
}

fn order_contains_nonce(order: &FillerOrder) -> bool {
    matches!(order, FillerOrder::AngstromOrder(AllOrders::Standing(_)))
        || matches!(order, FillerOrder::RegularOrder(_))
}

impl FillFrom<NonceGeneratorFiller, AllOrders> for Option<u64> {
    fn prepare_with(self, input_order: &mut AllOrders) -> eyre::Result<()> {
        match input_order {
            AllOrders::Standing(standing_variants) => match standing_variants {
                StandingVariants::Partial(partial_standing_order) => {
                    partial_standing_order.nonce = self.expect("expected nonce");
                }
                StandingVariants::Exact(exact_standing_order) => {
                    exact_standing_order.nonce = self.expect("expected nonce");
                }
            },
            _ => (),
        };

        Ok(())
    }
}

impl FillFrom<NonceGeneratorFiller, TransactionRequestWithLiquidityMeta> for Option<u64> {
    fn prepare_with(
        self,
        input_order: &mut TransactionRequestWithLiquidityMeta,
    ) -> eyre::Result<()> {
        input_order.tx_request.nonce = Some(self.expect("expected nonce"));
        Ok(())
    }
}
