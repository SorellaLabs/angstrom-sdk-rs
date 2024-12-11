mod balance_check;
pub use balance_check::*;

use crate::providers::{AngstromFillProvider, EthProvider};

pub(crate) trait AngstromFiller<O>: Sized {
    type FillOutput: FillFrom<O>;

    async fn fill<E: EthProvider>(&self, provider: &E, order: &mut O) -> eyre::Result<()> {
        let input = self.prepare(provider, &order).await?;
        input.prepare_with(order)?;
        Ok(())
    }

    async fn prepare<E: EthProvider>(
        &self,
        provider: &E,
        order: &O,
    ) -> eyre::Result<Self::FillOutput>;
}

impl<O> AngstromFiller<O> for () {
    type FillOutput = ();

    async fn prepare<E: EthProvider>(&self, _: &E, _: &O) -> eyre::Result<()> {
        Ok(())
    }
}

pub(crate) trait FillFrom<O> {
    fn prepare_with(self, input_order: &mut O) -> eyre::Result<()>;
}

impl<O> FillFrom<O> for () {
    fn prepare_with(self, _: &mut O) -> eyre::Result<()> {
        Ok(())
    }
}

pub(crate) trait FillWrapper<O>: AngstromFiller<O> {
    fn wrap_with_filler<F: AngstromFiller<O>>(self, filler: F) -> AngstromFillProvider<Self, F>;
}

impl<O> FillWrapper<O> for () {
    fn wrap_with_filler<F: AngstromFiller<O>>(self, filler: F) -> AngstromFillProvider<Self, F> {
        AngstromFillProvider::new(self, filler)
    }
}
