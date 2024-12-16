use crate::providers::{AngstromProvider, EthRpcProvider};
use crate::types::TransactionRequestWithLiquidityMeta;
use alloy_provider::Provider;
use alloy_transport::Transport;
use angstrom_types::sol_bindings::grouped_orders::AllOrders;

use super::{AngstromFiller, FillFrom, FillerOrder};

#[derive(Clone, Copy, Debug, Default)]
pub struct ChainIdFiller(u64);

impl ChainIdFiller {
    pub fn new(chain_id: u64) -> Self {
        Self(chain_id)
    }
}

impl AngstromFiller for ChainIdFiller {
    type FillOutput = Option<u64>;

    async fn prepare<P, T>(
        &self,
        _: &EthRpcProvider<P, T>,
        _: &AngstromProvider,
        order: &FillerOrder,
    ) -> eyre::Result<Self::FillOutput>
    where
        P: Provider<T>,
        T: Transport + Clone,
    {
        Ok(matches!(order, FillerOrder::RegularOrder(_)).then_some(self.0))
    }
}

impl FillFrom<ChainIdFiller, AllOrders> for Option<u64> {
    fn prepare_with(self, _: &mut AllOrders) -> eyre::Result<()> {
        Ok(())
    }
}

impl FillFrom<ChainIdFiller, TransactionRequestWithLiquidityMeta> for Option<u64> {
    fn prepare_with(
        self,
        input_order: &mut TransactionRequestWithLiquidityMeta,
    ) -> eyre::Result<()> {
        input_order.tx_request.chain_id = Some(self.expect("expected nonce"));
        Ok(())
    }
}
