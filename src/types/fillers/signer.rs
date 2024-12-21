use alloy_primitives::Address;
use alloy_provider::Provider;
use alloy_signer::{Signer, SignerSync};

use alloy_transport::Transport;
use angstrom_types::{
    primitive::ANGSTROM_DOMAIN,
    sol_bindings::{
        grouped_orders::{AllOrders, FlashVariants, StandingVariants},
        rpc_orders::{OmitOrderMeta, OrderMeta},
    },
};
use pade::PadeEncode;

use crate::{
    providers::{AngstromProvider, EthRpcProvider},
    types::TransactionRequestWithLiquidityMeta,
};

use super::{AngstromFiller, FillFrom, FillerOrder};

pub struct SignerFiller<S>(S);

impl<S: Signer + SignerSync> SignerFiller<S> {
    pub fn new(signer: S) -> Self {
        Self(signer)
    }

    fn sign_into_meta<O: OmitOrderMeta>(&self, order: &O) -> eyre::Result<OrderMeta> {
        let hash = order.no_meta_eip712_signing_hash(&ANGSTROM_DOMAIN);
        let sig = self.0.sign_hash_sync(&hash)?;
        Ok(OrderMeta {
            isEcdsa: true,
            from: self.0.address(),
            signature: sig.pade_encode().into(),
        })
    }
}

impl<S: Signer + SignerSync> AngstromFiller for SignerFiller<S> {
    type FillOutput = (Address, Option<OrderMeta>);

    async fn prepare<P, T>(
        &self,
        _: &EthRpcProvider<P, T>,
        _: &AngstromProvider,
        order: &FillerOrder,
    ) -> eyre::Result<Self::FillOutput>
    where
        P: Provider<T> + Clone,
        T: Transport + Clone,
    {
        let my_address = self.0.address();

        let order_meta = if let FillerOrder::AngstromOrder(fill_order) = order {
            let om = match fill_order {
                AllOrders::Standing(standing_variants) => match standing_variants {
                    StandingVariants::Partial(partial_standing_order) => {
                        self.sign_into_meta(partial_standing_order)?
                    }
                    StandingVariants::Exact(exact_standing_order) => {
                        self.sign_into_meta(exact_standing_order)?
                    }
                },
                AllOrders::Flash(flash_variants) => match flash_variants {
                    FlashVariants::Partial(partial_flash_order) => {
                        self.sign_into_meta(partial_flash_order)?
                    }
                    FlashVariants::Exact(exact_flash_order) => {
                        self.sign_into_meta(exact_flash_order)?
                    }
                },
                AllOrders::TOB(top_of_block_order) => self.sign_into_meta(top_of_block_order)?,
            };
            Some(om)
        } else {
            None
        };

        Ok((my_address, order_meta))
    }
}

impl<S: Signer + SignerSync> FillFrom<SignerFiller<S>, AllOrders> for (Address, Option<OrderMeta>) {
    fn prepare_with(self, input_order: &mut AllOrders) -> eyre::Result<()> {
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

impl<S: Signer + SignerSync> FillFrom<SignerFiller<S>, TransactionRequestWithLiquidityMeta>
    for (Address, Option<OrderMeta>)
{
    fn prepare_with(
        self,
        input_order: &mut TransactionRequestWithLiquidityMeta,
    ) -> eyre::Result<()> {
        input_order.tx_request.from = Some(self.0);
        Ok(())
    }
}
