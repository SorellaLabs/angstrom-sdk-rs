use crate::types::ANGSTROM_DOMAIN;
use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_signer::{Signer, SignerSync};
use angstrom_types::sol_bindings::{
    grouped_orders::{AllOrders, FlashVariants, StandingVariants},
    rpc_orders::{OmitOrderMeta, OrderMeta},
};
use pade::PadeEncode;

use super::{AngstromFiller, FillFrom, FillerOrder, FillerOrderFrom, errors::FillerError};
use crate::{providers::backend::AngstromProvider, types::TransactionRequestWithLiquidityMeta};

#[derive(Clone)]
pub struct AngstromSignerFiller<S>(S);

impl<S: Signer + SignerSync + Clone> AngstromSignerFiller<S> {
    pub fn new(signer: S) -> Self {
        Self(signer)
    }

    fn sign_into_meta<O: OmitOrderMeta>(&self, order: &O) -> Result<OrderMeta, FillerError> {
        let hash = order.no_meta_eip712_signing_hash(&ANGSTROM_DOMAIN);
        let sig = self.0.sign_hash_sync(&hash)?;
        Ok(OrderMeta { isEcdsa: true, from: self.0.address(), signature: sig.pade_encode().into() })
    }
}

impl<S: Signer + SignerSync + Clone> AngstromFiller for AngstromSignerFiller<S> {
    type FillOutput = (Address, Option<OrderMeta>);

    async fn prepare<P>(
        &self,
        _: &AngstromProvider<P>,
        order: &FillerOrderFrom,
    ) -> Result<Self::FillOutput, FillerError>
    where
        P: Provider,
    {
        let my_address = self.0.address();

        let order_meta = if let FillerOrder::AngstromOrder(fill_order) = &order.inner {
            let om = match fill_order {
                AllOrders::Standing(standing_variants) => match standing_variants {
                    StandingVariants::Partial(inner_order) => {
                        let mut inner_order = inner_order.clone();
                        inner_order.recipient = self.0.address();
                        self.sign_into_meta(&inner_order)?
                    }
                    StandingVariants::Exact(inner_order) => {
                        let mut inner_order = inner_order.clone();
                        inner_order.recipient = self.0.address();
                        self.sign_into_meta(&inner_order)?
                    }
                },
                AllOrders::Flash(flash_variants) => match flash_variants {
                    FlashVariants::Partial(inner_order) => {
                        let mut inner_order = inner_order.clone();
                        inner_order.recipient = self.0.address();
                        self.sign_into_meta(&inner_order)?
                    }
                    FlashVariants::Exact(inner_order) => {
                        let mut inner_order = inner_order.clone();
                        inner_order.recipient = self.0.address();
                        self.sign_into_meta(&inner_order)?
                    }
                },
                AllOrders::TOB(inner_order) => {
                    let mut inner_order = inner_order.clone();
                    inner_order.recipient = self.0.address();

                    self.sign_into_meta(&inner_order)?
                }
            };
            Some(om)
        } else {
            None
        };

        Ok((my_address, order_meta))
    }

    fn from(&self) -> Option<Address> {
        Some(self.0.address())
    }
}

impl<S: Signer + SignerSync + Clone> FillFrom<AngstromSignerFiller<S>, AllOrders>
    for (Address, Option<OrderMeta>)
{
    fn prepare_with(self, input_order: &mut AllOrders) -> Result<(), FillerError> {
        let (recipient, order_meta) = (self.0, self.1.expect("expected order meta"));
        match input_order {
            AllOrders::Standing(standing_variants) => match standing_variants {
                StandingVariants::Partial(partial_standing_order) => {
                    partial_standing_order.meta = order_meta;
                    partial_standing_order.recipient = recipient;
                }
                StandingVariants::Exact(exact_standing_order) => {
                    exact_standing_order.meta = order_meta;
                    exact_standing_order.recipient = recipient;
                }
            },
            AllOrders::Flash(flash_variants) => match flash_variants {
                FlashVariants::Partial(partial_flash_order) => {
                    partial_flash_order.meta = order_meta;
                    partial_flash_order.recipient = recipient;
                }
                FlashVariants::Exact(exact_flash_order) => {
                    exact_flash_order.meta = order_meta;
                    exact_flash_order.recipient = recipient;
                }
            },
            AllOrders::TOB(top_of_block_order) => {
                top_of_block_order.meta = order_meta;
                top_of_block_order.recipient = recipient;
            }
        };

        Ok(())
    }
}

impl<S: Signer + SignerSync + Clone>
    FillFrom<AngstromSignerFiller<S>, TransactionRequestWithLiquidityMeta>
    for (Address, Option<OrderMeta>)
{
    fn prepare_with(
        self,
        input_order: &mut TransactionRequestWithLiquidityMeta,
    ) -> Result<(), FillerError> {
        input_order.tx_request.from = Some(self.0);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy_signer_local::LocalSigner;

    use crate::{
        AngstromApi,
        test_utils::filler_orders::{AllOrdersSpecific, AnvilAngstromProvider},
        types::fillers::MakeFillerOrder,
    };

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_signer_angstrom_order() {
        let signer = LocalSigner::random();
        let provider = AnvilAngstromProvider::new().await.unwrap();
        let api = AngstromApi::new_with_provider(provider.provider.clone())
            .with_angstrom_signer_filler(signer.clone());

        let orders = AllOrdersSpecific::default();

        let sig_f = |hash| {
            let sig = signer.sign_hash_sync(&hash).unwrap();
            OrderMeta { isEcdsa: true, from: signer.address(), signature: sig.pade_encode().into() }
        };

        let ref_api = &api;
        orders
            .test_filler_order(async |mut order| {
                let mut inner_order = order.clone().convert_with_from(signer.address());
                ref_api.fill(&mut inner_order).await.unwrap();

                match &mut order {
                    AllOrders::Standing(StandingVariants::Exact(inner_order)) => {
                        let hash = inner_order.no_meta_eip712_signing_hash(&ANGSTROM_DOMAIN);
                        inner_order.meta = sig_f(hash);
                        inner_order.recipient = signer.address();
                    }
                    AllOrders::Standing(StandingVariants::Partial(inner_order)) => {
                        let hash = inner_order.no_meta_eip712_signing_hash(&ANGSTROM_DOMAIN);
                        inner_order.meta = sig_f(hash);
                        inner_order.recipient = signer.address();
                    }
                    AllOrders::Flash(FlashVariants::Exact(inner_order)) => {
                        let hash = inner_order.no_meta_eip712_signing_hash(&ANGSTROM_DOMAIN);
                        inner_order.meta = sig_f(hash);
                        inner_order.recipient = signer.address();
                    }
                    AllOrders::Flash(FlashVariants::Partial(inner_order)) => {
                        let hash = inner_order.no_meta_eip712_signing_hash(&ANGSTROM_DOMAIN);
                        inner_order.meta = sig_f(hash);
                        inner_order.recipient = signer.address();
                    }
                    AllOrders::TOB(inner_order) => {
                        let hash = inner_order.no_meta_eip712_signing_hash(&ANGSTROM_DOMAIN);
                        inner_order.meta = sig_f(hash);
                        inner_order.recipient = signer.address();
                    }
                }

                inner_order.inner.force_angstrom_order() == order
            })
            .await;
    }
}
