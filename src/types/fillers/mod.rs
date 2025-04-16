mod balance_check;
pub mod errors;
use alloy_primitives::Address;
use alloy_provider::Provider;
use angstrom_types::{primitive::AngstromSigner, sol_bindings::grouped_orders::AllOrders};
pub use balance_check::*;
mod signer;
use errors::FillerError;
pub use signer::*;
mod nonce_generator;
pub use nonce_generator::*;
mod chain_id;
pub use chain_id::*;

use super::TransactionRequestWithLiquidityMeta;
use crate::providers::backend::AngstromProvider;

#[derive(Clone)]
pub struct AngstromFillProvider<L, R> {
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

    async fn fill<P>(
        &self,
        provider: &AngstromProvider<P>,
        order: &mut FillerOrderFrom,
    ) -> Result<(), FillerError>
    where
        P: Provider,
    {
        self.left.fill(provider, order).await?;
        self.right.fill(provider, order).await?;

        order.maybe_fill_modify_liquidity_call();

        Ok(())
    }

    async fn prepare<P>(
        &self,
        _: &AngstromProvider<P>,
        _: &FillerOrderFrom,
    ) -> Result<(), FillerError>
    where
        P: Provider,
    {
        Ok(())
    }

    fn from(&self) -> Option<Address> {
        if let Some(l) = self.left.from() { Some(l) } else { self.right.from() }
    }

    fn angstrom_signer(&self) -> Option<&AngstromSigner> {
        if let Some(l) = self.left.angstrom_signer() {
            Some(l)
        } else {
            self.right.angstrom_signer()
        }
    }
}

impl<L: AngstromFiller, R: AngstromFiller> FillWrapper for AngstromFillProvider<L, R> {}

pub(crate) trait AngstromFiller: Clone + Sized {
    type FillOutput: FillFrom<Self, AllOrders> + FillFrom<Self, TransactionRequestWithLiquidityMeta>;

    async fn fill<P>(
        &self,
        provider: &AngstromProvider<P>,
        order: &mut FillerOrderFrom,
    ) -> Result<(), FillerError>
    where
        P: Provider,
    {
        let input = self.prepare(provider, order).await?;
        match &mut order.inner {
            FillerOrder::AngstromOrder(all_orders) => input.prepare_with(all_orders)?,
            FillerOrder::EthOrder(tx_request) => input.prepare_with(tx_request)?,
        }

        Ok(())
    }

    async fn fill_many<P>(
        &self,
        provider: &AngstromProvider<P>,
        orders: &mut [FillerOrderFrom],
    ) -> Result<(), FillerError>
    where
        P: Provider,
    {
        let inputs = self.prepare_many(provider, orders).await;

        for (order, input) in orders.iter_mut().zip(inputs) {
            let input = input?;
            match &mut order.inner {
                FillerOrder::AngstromOrder(all_orders) => input.prepare_with(all_orders)?,
                FillerOrder::EthOrder(tx_request) => input.prepare_with(tx_request)?,
            }
        }

        Ok(())
    }

    async fn prepare<P>(
        &self,
        provider: &AngstromProvider<P>,
        order: &FillerOrderFrom,
    ) -> Result<Self::FillOutput, FillerError>
    where
        P: Provider;

    async fn prepare_many<P>(
        &self,
        provider: &AngstromProvider<P>,
        orders: &[FillerOrderFrom],
    ) -> Vec<Result<Self::FillOutput, FillerError>>
    where
        P: Provider,
    {
        futures::future::join_all(orders.iter().map(|order| self.prepare(provider, order))).await
    }

    fn from(&self) -> Option<Address> {
        None
    }

    fn angstrom_signer(&self) -> Option<&AngstromSigner> {
        None
    }
}

impl AngstromFiller for () {
    type FillOutput = ();

    async fn prepare<P>(
        &self,
        _: &AngstromProvider<P>,
        _: &FillerOrderFrom,
    ) -> Result<(), FillerError>
    where
        P: Provider,
    {
        Ok(())
    }

    fn from(&self) -> Option<Address> {
        None
    }
}

pub trait FillWrapper: AngstromFiller + Clone {
    fn wrap_with_filler<F: AngstromFiller>(self, filler: F) -> AngstromFillProvider<Self, F> {
        AngstromFillProvider::new(self, filler)
    }
}

impl FillWrapper for () {}

pub(crate) trait FillFrom<F: AngstromFiller, O> {
    fn prepare_with(self, input_order: &mut O) -> Result<(), FillerError>;
}

impl<F: AngstromFiller, O> FillFrom<F, O> for () {
    fn prepare_with(self, _: &mut O) -> Result<(), FillerError> {
        Ok(())
    }
}

pub(crate) struct FillerOrderFrom {
    pub from: Address,
    pub inner: FillerOrder,
}

impl FillerOrderFrom {
    fn maybe_fill_modify_liquidity_call(&mut self) {
        match &mut self.inner {
            FillerOrder::EthOrder(call) => call.fill_modify_liquidity_call(),
            _ => (),
        }
    }
}

pub(crate) enum FillerOrder {
    AngstromOrder(AllOrders),
    EthOrder(TransactionRequestWithLiquidityMeta),
}

impl FillerOrder {
    pub(crate) fn force_all_orders(self) -> AllOrders {
        match self {
            FillerOrder::AngstromOrder(o) => o,
            _ => unreachable!(),
        }
    }

    pub(crate) fn force_regular_tx(self) -> TransactionRequestWithLiquidityMeta {
        match self {
            FillerOrder::EthOrder(o) => o,
            _ => unreachable!(),
        }
    }
}

impl From<AllOrders> for FillerOrder {
    fn from(value: AllOrders) -> Self {
        FillerOrder::AngstromOrder(value)
    }
}

impl From<TransactionRequestWithLiquidityMeta> for FillerOrder {
    fn from(value: TransactionRequestWithLiquidityMeta) -> Self {
        FillerOrder::EthOrder(value)
    }
}

pub(crate) trait MakeFillerOrder: Sized {
    fn convert_with_from(self, from: Address) -> FillerOrderFrom;
}

impl MakeFillerOrder for TransactionRequestWithLiquidityMeta {
    fn convert_with_from(self, from: Address) -> FillerOrderFrom {
        FillerOrderFrom { from, inner: self.into() }
    }
}

impl MakeFillerOrder for AllOrders {
    fn convert_with_from(self, from: Address) -> FillerOrderFrom {
        FillerOrderFrom { from, inner: self.into() }
    }
}
