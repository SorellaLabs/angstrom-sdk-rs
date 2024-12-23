use super::{AngstromFiller, FillFrom, FillerOrder};
use crate::apis::node_api::AngstromNodeApi;
use crate::providers::AngstromProvider;
use crate::providers::EthRpcProvider;
use crate::types::TransactionRequestWithLiquidityMeta;
use alloy_provider::Provider;
use alloy_transport::Transport;
use angstrom_types::sol_bindings::grouped_orders::AllOrders;
use angstrom_types::sol_bindings::grouped_orders::StandingVariants;

#[derive(Clone, Copy, Debug, Default)]
pub struct NonceGeneratorFiller;

impl AngstromFiller for NonceGeneratorFiller {
    type FillOutput = Option<u64>;

    async fn prepare<P, T>(
        &self,
        provider: &EthRpcProvider<P, T>,
        angstrom_provider: &AngstromProvider,
        order: &FillerOrder,
    ) -> eyre::Result<Self::FillOutput>
    where
        P: Provider<T> + Clone,
        T: Transport + Clone,
    {
        if !order_contains_nonce(order) {
            return Ok(None);
        }

        let Some(from) = order.from() else {
            return Ok(None);
        };

        let current_nonce = provider.provider().get_transaction_count(from).await?;
        let pending_orders = angstrom_provider.pending_orders(vec![from]).await?.len() as u64;

        Ok(Some(current_nonce + pending_orders + 1))
    }

    // probably need different function for this
    // async fn prepare_many<P, T>(
    //     &self,
    //     provider: &EthRpcProvider<P, T>,
    //     angstrom_provider: &AngstromProvider,
    //     orders: &[FillerOrder],
    // ) -> eyre::Result<Vec<Self::FillOutput>>
    // where
    //     P: Provider<T> + Clone,
    //     T: Transport + Clone,
    // {

    // }
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
        input_order.tx_request.nonce = self;
        Ok(())
    }
}
