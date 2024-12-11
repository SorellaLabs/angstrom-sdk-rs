use crate::types::fillers::{AngstromFiller, FillWrapper, FillerOrder};

use super::{AngstromProvider, EthProvider};

pub struct AngstromFillProvider<L, R> {
    left: L,
    right: R,
}

impl<L, R> AngstromFillProvider<L, R> {
    pub fn new(left: L, right: R) -> Self {
        Self { left, right }
    }
}

impl<L, R> AngstromFiller for AngstromFillProvider<L, R>
where
    L: AngstromFiller,
    R: AngstromFiller,
{
    type FillOutput = ();

    async fn fill<E: EthProvider>(
        &self,
        eth_provider: &E,
        angstrom_provider: &AngstromProvider,
        order: &mut FillerOrder,
    ) -> eyre::Result<()> {
        self.left
            .fill(eth_provider, angstrom_provider, order)
            .await?;
        self.right
            .fill(eth_provider, angstrom_provider, order)
            .await?;

        Ok(())
    }

    async fn prepare<E: EthProvider>(
        &self,
        _: &E,
        _: &AngstromProvider,
        _: &FillerOrder,
    ) -> eyre::Result<()> {
        Ok(())
    }
}

impl<L, R> FillWrapper for AngstromFillProvider<L, R>
where
    L: AngstromFiller,
    R: AngstromFiller,
{
    fn wrap_with_filler<F: AngstromFiller>(self, filler: F) -> AngstromFillProvider<Self, F> {
        AngstromFillProvider::new(self, filler)
    }
}
