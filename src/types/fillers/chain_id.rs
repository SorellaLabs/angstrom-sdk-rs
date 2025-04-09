use alloy_provider::Provider;
use angstrom_types::{CHAIN_ID, sol_bindings::grouped_orders::AllOrders};

use super::{AngstromFiller, FillFrom, FillerOrder, errors::FillerError};
use crate::{providers::AngstromProvider, types::TransactionRequestWithLiquidityMeta};

#[derive(Clone, Copy, Debug, Default)]
pub struct ChainIdFiller;

impl AngstromFiller for ChainIdFiller {
    type FillOutput = Option<u64>;

    async fn prepare<P>(
        &self,
        _: &AngstromProvider<P>,
        _: &FillerOrder,
    ) -> Result<Self::FillOutput, FillerError>
    where
        P: Provider,
    {
        Ok(None)
    }
}

impl FillFrom<ChainIdFiller, AllOrders> for Option<u64> {
    fn prepare_with(self, _: &mut AllOrders) -> Result<(), FillerError> {
        Ok(())
    }
}

impl FillFrom<ChainIdFiller, TransactionRequestWithLiquidityMeta> for Option<u64> {
    fn prepare_with(
        self,
        input_order: &mut TransactionRequestWithLiquidityMeta,
    ) -> Result<(), FillerError> {
        input_order.tx_request.chain_id = Some(CHAIN_ID);
        Ok(())
    }
}
