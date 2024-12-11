use crate::types::fillers::{AngstromFiller, FillWrapper};

use super::EthProvider;

pub struct AngstromFillProvider<L, R> {
    left: L,
    right: R,
}

impl<L, R> AngstromFillProvider<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<O, L, R> AngstromFiller<O> for AngstromFillProvider<L, R>
where
    L: AngstromFiller<O>,
    R: AngstromFiller<O>,
{
    type FillOutput = ();

    async fn fill<E: EthProvider>(&self, provider: &E, order: &mut O) -> eyre::Result<()> {
        self.left.fill(provider, order).await?;
        self.right.fill(provider, order).await?;

        Ok(())
    }

    async fn prepare<E: EthProvider>(&self, _: &E, _: &O) -> eyre::Result<()> {
        Ok(())
    }
}

impl<O, L, R> FillWrapper<O> for AngstromFillProvider<L, R>
where
    L: AngstromFiller<O>,
    R: AngstromFiller<O>,
{
    fn wrap_with_filler<F: AngstromFiller<O>>(self, filler: F) -> AngstromFillProvider<Self, F> {
        AngstromFillProvider::new(self, filler)
    }
}
