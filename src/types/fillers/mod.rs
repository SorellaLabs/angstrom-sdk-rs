mod balance_check;

use alloy_provider::Provider;
use alloy_transport::Transport;
use angstrom_types::sol_bindings::grouped_orders::AllOrders;
pub use balance_check::*;
mod signer;
pub use signer::*;
mod nonce_generator;
pub use nonce_generator::*;
mod chain_id;
pub use chain_id::*;

use crate::providers::{AngstromProvider, EthRpcProvider};

use super::TransactionRequestWithLiquidityMeta;

pub(crate) struct AngstromFillProvider<L, R> {
    left: L,
    right: R,
}

impl<L, R> AngstromFillProvider<L, R> {
    pub(crate) fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L, R> AngstromFiller for AngstromFillProvider<L, R>
where
    L: AngstromFiller,
    R: AngstromFiller,
{
    type FillOutput = ();

    async fn fill<P, T>(
        &self,
        provider: &EthRpcProvider<P, T>,
        angstrom_provider: &AngstromProvider,
        order: &mut FillerOrder,
    ) -> eyre::Result<()>
    where
        P: Provider<T>,
        T: Transport + Clone,
    {
        self.left.fill(provider, angstrom_provider, order).await?;
        self.right.fill(provider, angstrom_provider, order).await?;

        Ok(())
    }

    async fn prepare<P, T>(
        &self,
        _: &EthRpcProvider<P, T>,
        _: &AngstromProvider,
        _: &FillerOrder,
    ) -> eyre::Result<()>
    where
        P: Provider<T>,
        T: Transport + Clone,
    {
        Ok(())
    }
}

impl<L: AngstromFiller, R: AngstromFiller> FillWrapper for AngstromFillProvider<L, R> {}

pub(crate) trait AngstromFiller: Sized {
    type FillOutput: FillFrom<Self, AllOrders> + FillFrom<Self, TransactionRequestWithLiquidityMeta>;

    async fn fill<P, T>(
        &self,
        provider: &EthRpcProvider<P, T>,
        angstrom_provider: &AngstromProvider,
        order: &mut FillerOrder,
    ) -> eyre::Result<()>
    where
        P: Provider<T>,
        T: Transport + Clone,
    {
        let input = self.prepare(provider, angstrom_provider, &order).await?;
        match order {
            FillerOrder::AngstromOrder(all_orders) => input.prepare_with(all_orders)?,
            FillerOrder::RegularOrder(tx_request) => input.prepare_with(tx_request)?,
        }

        Ok(())
    }

    async fn prepare<P, T>(
        &self,
        provider: &EthRpcProvider<P, T>,
        angstrom_provider: &AngstromProvider,
        order: &FillerOrder,
    ) -> eyre::Result<Self::FillOutput>
    where
        P: Provider<T>,
        T: Transport + Clone;
}

impl AngstromFiller for () {
    type FillOutput = ();

    async fn prepare<P, T>(
        &self,
        _: &EthRpcProvider<P, T>,
        _: &AngstromProvider,
        _: &FillerOrder,
    ) -> eyre::Result<()>
    where
        P: Provider<T>,
        T: Transport + Clone,
    {
        Ok(())
    }
}

pub(crate) trait FillWrapper: AngstromFiller {
    fn wrap_with_filler<F: AngstromFiller>(self, filler: F) -> AngstromFillProvider<Self, F> {
        AngstromFillProvider::new(self, filler)
    }
}

impl FillWrapper for () {}

pub(crate) trait FillFrom<F: AngstromFiller, O> {
    fn prepare_with(self, input_order: &mut O) -> eyre::Result<()>;
}

impl<F: AngstromFiller, O> FillFrom<F, O> for () {
    fn prepare_with(self, _: &mut O) -> eyre::Result<()> {
        Ok(())
    }
}

pub(crate) enum FillerOrder {
    AngstromOrder(AllOrders),
    RegularOrder(TransactionRequestWithLiquidityMeta),
}

impl From<AllOrders> for FillerOrder {
    fn from(value: AllOrders) -> Self {
        FillerOrder::AngstromOrder(value)
    }
}

impl From<TransactionRequestWithLiquidityMeta> for FillerOrder {
    fn from(value: TransactionRequestWithLiquidityMeta) -> Self {
        FillerOrder::RegularOrder(value)
    }
}

/*

 TODO
  - gas price calc
  - gas estimator

*/
