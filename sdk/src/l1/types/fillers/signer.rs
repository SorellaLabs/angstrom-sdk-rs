use alloy_primitives::Address;
use alloy_signer::{Signer, SignerSync};
use angstrom_types_primitives::{
    primitive::ANGSTROM_DOMAIN,
    sol_bindings::{
        grouped_orders::AllOrders,
        rpc_orders::{OmitOrderMeta, OrderMeta}
    }
};
use pade::PadeEncode;

use super::{FillFrom, FillWrapper, errors::FillerError};
use crate::l1::{apis::node_api::AngstromOrderApiClient, providers::backend::AngstromProvider};

#[derive(Clone)]
pub struct AngstromSignerFiller<S>(S);

impl<S: Signer + SignerSync + Clone> AngstromSignerFiller<S> {
    pub fn new(signer: S) -> Self {
        Self(signer)
    }

    fn sign_into_meta<O: OmitOrderMeta>(&self, order: &O) -> Result<OrderMeta, FillerError> {
        let hash = order.no_meta_eip712_signing_hash(ANGSTROM_DOMAIN.get().unwrap());
        let sig = self.0.sign_hash_sync(&hash)?;

        Ok(OrderMeta {
            isEcdsa:   true,
            from:      self.0.address(),
            signature: sig.pade_encode().into()
        })
    }
}

#[async_trait::async_trait]
impl<S: Signer + SignerSync + Send + Sync + Clone> FillWrapper for AngstromSignerFiller<S> {
    type FillOutput = (Address, OrderMeta);

    async fn prepare<T>(
        &self,
        _: &AngstromProvider<T>,
        order: &AllOrders
    ) -> Result<Self::FillOutput, FillerError>
    where
        T: AngstromOrderApiClient
    {
        let my_address = self.0.address();

        let om = match order {
            AllOrders::PartialStanding(inner_order) => {
                let mut inner_order = inner_order.clone();
                inner_order.recipient = self.0.address();
                self.sign_into_meta(&inner_order)?
            }
            AllOrders::ExactStanding(inner_order) => {
                let mut inner_order = inner_order.clone();
                inner_order.recipient = self.0.address();
                self.sign_into_meta(&inner_order)?
            }
            AllOrders::PartialFlash(inner_order) => {
                let mut inner_order = inner_order.clone();
                inner_order.recipient = self.0.address();
                self.sign_into_meta(&inner_order)?
            }
            AllOrders::ExactFlash(inner_order) => {
                let mut inner_order = inner_order.clone();
                inner_order.recipient = self.0.address();
                self.sign_into_meta(&inner_order)?
            }
            AllOrders::TOB(inner_order) => {
                let mut inner_order = inner_order.clone();
                inner_order.recipient = self.0.address();
                self.sign_into_meta(&inner_order)?
            }
        };

        Ok((my_address, om))
    }

    fn from(&self) -> Option<Address> {
        Some(self.0.address())
    }
}

impl<S: Signer + SignerSync + Send + Sync + Clone> FillFrom<AngstromSignerFiller<S>>
    for (Address, OrderMeta)
{
    fn prepare_with(self, input_order: &mut AllOrders) -> Result<(), FillerError> {
        let (recipient, order_meta) = (self.0, self.1);
        match input_order {
            AllOrders::PartialStanding(inner_order) => {
                inner_order.meta = order_meta;
                inner_order.recipient = recipient;
            }
            AllOrders::ExactStanding(inner_order) => {
                inner_order.meta = order_meta;
                inner_order.recipient = recipient;
            }
            AllOrders::PartialFlash(inner_order) => {
                inner_order.meta = order_meta;
                inner_order.recipient = recipient;
            }
            AllOrders::ExactFlash(inner_order) => {
                inner_order.meta = order_meta;
                inner_order.recipient = recipient;
            }
            AllOrders::TOB(top_of_block_order) => {
                top_of_block_order.meta = order_meta;
                top_of_block_order.recipient = recipient;
            }
        };

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy_signer_local::LocalSigner;
    use angstrom_types_primitives::primitive::try_init_with_chain_id;

    use super::*;
    use crate::l1::{
        AngstromApi,
        test_utils::filler_orders::{AllOrdersSpecific, AnvilAngstromProvider}
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn test_signer_angstrom_order() {
        let _ = try_init_with_chain_id(1);

        let signer = LocalSigner::random();
        let provider = AnvilAngstromProvider::new().await.unwrap();
        let api = AngstromApi::new_with_provider(provider.provider)
            .with_angstrom_signer_filler(signer.clone());

        let orders = AllOrdersSpecific::default();

        let sig_f = |hash| {
            let sig = signer.sign_hash_sync(&hash).unwrap();
            OrderMeta {
                isEcdsa:   true,
                from:      signer.address(),
                signature: sig.pade_encode().into()
            }
        };

        let ref_api = &api;
        orders
            .test_filler_order(async |mut order| {
                let mut inner_order = order.clone();
                ref_api.fill(&mut inner_order).await.unwrap();

                let domain = ANGSTROM_DOMAIN.get().expect("ANGSTROM_DOMAIN not set");
                match &mut order {
                    AllOrders::ExactStanding(inner_order) => {
                        inner_order.recipient = signer.address();
                        let hash = inner_order.no_meta_eip712_signing_hash(domain);
                        inner_order.meta = sig_f(hash);
                    }
                    AllOrders::PartialStanding(inner_order) => {
                        inner_order.recipient = signer.address();
                        let hash = inner_order.no_meta_eip712_signing_hash(domain);
                        inner_order.meta = sig_f(hash);
                    }
                    AllOrders::ExactFlash(inner_order) => {
                        inner_order.recipient = signer.address();
                        let hash = inner_order.no_meta_eip712_signing_hash(domain);
                        inner_order.meta = sig_f(hash);
                    }
                    AllOrders::PartialFlash(inner_order) => {
                        inner_order.recipient = signer.address();
                        let hash = inner_order.no_meta_eip712_signing_hash(domain);
                        inner_order.meta = sig_f(hash);

                        inner_order.recipient = signer.address();
                    }
                    AllOrders::TOB(inner_order) => {
                        inner_order.recipient = signer.address();
                        let hash = inner_order.no_meta_eip712_signing_hash(domain);
                        inner_order.meta = sig_f(hash);
                    }
                }

                inner_order == order
            })
            .await;
    }
}
