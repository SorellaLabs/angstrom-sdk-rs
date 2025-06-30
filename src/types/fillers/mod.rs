mod balance_check;
pub mod errors;
use alloy_primitives::Address;
use alloy_provider::Provider;
use angstrom_types::sol_bindings::grouped_orders::AllOrders;
pub use balance_check::*;
mod signer;
use errors::FillerError;
use futures::FutureExt;
pub use signer::*;
mod nonce_generator;
pub use nonce_generator::*;

use crate::{apis::node_api::AngstromOrderApiClient, providers::backend::AngstromProvider};

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

#[async_trait::async_trait]
impl<L, R> FillWrapper for AngstromFillProvider<L, R>
where
    L: FillWrapper,
    R: FillWrapper,
{
    type FillOutput = ();

    async fn fill<P, T>(
        &self,
        provider: &AngstromProvider<P, T>,
        order: &mut AllOrders,
    ) -> Result<(), FillerError>
    where
        P: Provider,
        T: AngstromOrderApiClient,
    {
        self.left.fill(provider, order).await?;
        self.right.fill(provider, order).await?;

        Ok(())
    }

    async fn prepare<P, T>(
        &self,
        _: &AngstromProvider<P, T>,
        _: &AllOrders,
    ) -> Result<(), FillerError>
    where
        P: Provider,
        T: AngstromOrderApiClient,
    {
        Ok(())
    }

    fn from(&self) -> Option<Address> {
        if let Some(l) = self.left.from() { Some(l) } else { self.right.from() }
    }
}

impl<L: FillWrapper, R: FillWrapper> AngstromFiller for AngstromFillProvider<L, R> {}

#[async_trait::async_trait]
pub(crate) trait FillWrapper: Send + Sync + Clone + Sized {
    type FillOutput: FillFrom<Self>;

    async fn fill<P, T>(
        &self,
        provider: &AngstromProvider<P, T>,
        order: &mut AllOrders,
    ) -> Result<(), FillerError>
    where
        P: Provider,
        T: AngstromOrderApiClient,
    {
        let input = self.prepare(provider, order).await?;
        input.prepare_with(order)?;

        Ok(())
    }

    async fn fill_many<P, T>(
        &self,
        provider: &AngstromProvider<P, T>,
        orders: &mut [AllOrders],
    ) -> Result<(), FillerError>
    where
        P: Provider,
        T: AngstromOrderApiClient,
    {
        let inputs = self.prepare_many(provider, orders).await;

        for (order, input) in orders.iter_mut().zip(inputs) {
            let input = input?;
            input.prepare_with(order)?;
        }

        Ok(())
    }

    async fn prepare<P, T>(
        &self,
        provider: &AngstromProvider<P, T>,
        order: &AllOrders,
    ) -> Result<Self::FillOutput, FillerError>
    where
        P: Provider,
        T: AngstromOrderApiClient;

    async fn prepare_many<P, T>(
        &self,
        provider: &AngstromProvider<P, T>,
        orders: &[AllOrders],
    ) -> Vec<Result<Self::FillOutput, FillerError>>
    where
        P: Provider,
        T: AngstromOrderApiClient,
    {
        futures::future::join_all(
            orders
                .iter()
                .map(|order| self.prepare(provider, order).boxed()),
        )
        .await
    }

    fn from(&self) -> Option<Address> {
        None
    }
}

#[async_trait::async_trait]
impl FillWrapper for () {
    type FillOutput = ();

    async fn prepare<P, T>(
        &self,
        _: &AngstromProvider<P, T>,
        _: &AllOrders,
    ) -> Result<(), FillerError>
    where
        P: Provider,
        T: AngstromOrderApiClient,
    {
        Ok(())
    }
}

pub trait AngstromFiller: FillWrapper {
    fn wrap_with_filler<F: FillWrapper>(self, filler: F) -> AngstromFillProvider<Self, F> {
        AngstromFillProvider::new(self, filler)
    }
}

impl AngstromFiller for () {}

pub(crate) trait FillFrom<F: FillWrapper>: Send + Sync {
    fn prepare_with(self, input_order: &mut AllOrders) -> Result<(), FillerError>;
}

impl<F: FillWrapper> FillFrom<F> for () {
    fn prepare_with(self, _: &mut AllOrders) -> Result<(), FillerError> {
        Ok(())
    }
}
