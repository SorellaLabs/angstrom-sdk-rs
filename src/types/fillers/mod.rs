mod balance_check;
pub mod errors;
use alloy_primitives::Address;
use alloy_provider::Provider;
use angstrom_types::sol_bindings::grouped_orders::AllOrders;
pub use balance_check::*;
mod signer;
use errors::FillerError;
pub use signer::*;
mod nonce_generator;
pub use nonce_generator::*;

use crate::providers::backend::AngstromProvider;

#[derive(Clone)]
pub struct AngstromFillProvider<L, R> {
    left:  L,
    right: R
}

impl<L, R> AngstromFillProvider<L, R> {
    pub(crate) fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L, R> AngstromFiller for AngstromFillProvider<L, R>
where
    L: AngstromFiller,
    R: AngstromFiller
{
    type FillOutput = ();

    async fn fill<P>(
        &self,
        provider: &AngstromProvider<P>,
        order: &mut AllOrders
    ) -> Result<(), FillerError>
    where
        P: Provider
    {
        self.left.fill(provider, order).await?;
        self.right.fill(provider, order).await?;

        Ok(())
    }

    async fn prepare<P>(&self, _: &AngstromProvider<P>, _: &AllOrders) -> Result<(), FillerError>
    where
        P: Provider
    {
        Ok(())
    }

    fn from(&self) -> Option<Address> {
        if let Some(l) = self.left.from() { Some(l) } else { self.right.from() }
    }
}

impl<L: AngstromFiller, R: AngstromFiller> FillWrapper for AngstromFillProvider<L, R> {}

pub(crate) trait AngstromFiller: Clone + Sized {
    type FillOutput: FillFrom<Self>;

    async fn fill<P>(
        &self,
        provider: &AngstromProvider<P>,
        order: &mut AllOrders
    ) -> Result<(), FillerError>
    where
        P: Provider
    {
        let input = self.prepare(provider, order).await?;
        input.prepare_with(order)?;

        Ok(())
    }

    async fn fill_many<P>(
        &self,
        provider: &AngstromProvider<P>,
        orders: &mut [AllOrders]
    ) -> Result<(), FillerError>
    where
        P: Provider
    {
        let inputs = self.prepare_many(provider, orders).await;

        for (order, input) in orders.iter_mut().zip(inputs) {
            let input = input?;
            input.prepare_with(order)?;
        }

        Ok(())
    }

    async fn prepare<P>(
        &self,
        provider: &AngstromProvider<P>,
        order: &AllOrders
    ) -> Result<Self::FillOutput, FillerError>
    where
        P: Provider;

    async fn prepare_many<P>(
        &self,
        provider: &AngstromProvider<P>,
        orders: &[AllOrders]
    ) -> Vec<Result<Self::FillOutput, FillerError>>
    where
        P: Provider
    {
        futures::future::join_all(orders.iter().map(|order| self.prepare(provider, order))).await
    }

    fn from(&self) -> Option<Address> {
        None
    }
}

impl AngstromFiller for () {
    type FillOutput = ();

    async fn prepare<P>(&self, _: &AngstromProvider<P>, _: &AllOrders) -> Result<(), FillerError>
    where
        P: Provider
    {
        Ok(())
    }
}

pub trait FillWrapper: AngstromFiller + Clone {
    fn wrap_with_filler<F: AngstromFiller>(self, filler: F) -> AngstromFillProvider<Self, F> {
        AngstromFillProvider::new(self, filler)
    }
}

impl FillWrapper for () {}

pub(crate) trait FillFrom<F: AngstromFiller> {
    fn prepare_with(self, input_order: &mut AllOrders) -> Result<(), FillerError>;
}

impl<F: AngstromFiller> FillFrom<F> for () {
    fn prepare_with(self, _: &mut AllOrders) -> Result<(), FillerError> {
        Ok(())
    }
}
