use super::{AngstromFiller, FillFrom, FillerOrder, FillerOrderFrom, errors::FillerError};
use crate::{
    providers::AngstromProvider,
    types::{ANGSTROM_ADDRESS, TransactionRequestWithLiquidityMeta},
};
use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use angstrom_types::sol_bindings::grouped_orders::{AllOrders, StandingVariants};

use validation::order::state::db_state_utils::nonces::Nonces;

#[derive(Clone, Copy, Debug, Default)]
pub struct NonceGeneratorFiller;

impl NonceGeneratorFiller {
    async fn get_valid_nonce<P: Provider>(user: Address, provider: &P) -> Result<u64, FillerError> {
        let nonce_tracker = Nonces::new(ANGSTROM_ADDRESS);

        let mut nonce: u64 = rand::random();
        loop {
            let slot = nonce_tracker.get_nonce_word_slot(user, nonce);

            let word = provider
                .get_storage_at(ANGSTROM_ADDRESS, slot.into())
                .await?;

            let flag = U256::from(1) << (nonce as u8);

            if (word ^ flag) & flag == flag {
                break Ok(nonce);
            } else {
                nonce = rand::random();
            }
        }
    }

    async fn prepare_angstrom_order<P>(
        &self,
        provider: &AngstromProvider<P>,
        order: &AllOrders,
        from: Address,
    ) -> Result<<Self as AngstromFiller>::FillOutput, FillerError>
    where
        P: Provider,
    {
        if !matches!(order, AllOrders::Standing(_)) {
            return Ok(None);
        }

        let nonce = Self::get_valid_nonce(from, provider.eth_provider()).await?;

        Ok(Some(nonce))
    }

    async fn prepare_eth_order<P>(
        &self,
        provider: &AngstromProvider<P>,
        from: Address,
    ) -> Result<<Self as AngstromFiller>::FillOutput, FillerError>
    where
        P: Provider,
    {
        let nonce = provider.eth_provider().get_transaction_count(from).await?;
        Ok(Some(nonce))
    }
}

impl AngstromFiller for NonceGeneratorFiller {
    type FillOutput = Option<u64>;

    async fn prepare<P>(
        &self,
        provider: &AngstromProvider<P>,
        order: &FillerOrderFrom,
    ) -> Result<Self::FillOutput, FillerError>
    where
        P: Provider,
    {
        match &order.inner {
            FillerOrder::AngstromOrder(inner_order) => {
                self.prepare_angstrom_order(provider, inner_order, order.from)
                    .await
            }
            FillerOrder::EthOrder(_) => self.prepare_eth_order(provider, order.from).await,
        }
    }

    // probably need different function for this
    // async fn prepare_many<P, T>(
    //     &self,
    //     provider: &AngstromProvider<P, T>,
    //     angstrom_provider: &AngstromProvider,
    //     orders: &[FillerOrderFrom],
    // ) -> eyre::Result<Vec<Self::FillOutput>>
    // where
    //     P: Provider<T> + Clone,
    //     T: Transport + Clone,
    // {

    // }
}

impl FillFrom<NonceGeneratorFiller, AllOrders> for Option<u64> {
    fn prepare_with(self, input_order: &mut AllOrders) -> Result<(), FillerError> {
        if let AllOrders::Standing(standing_variants) = input_order {
            match standing_variants {
                StandingVariants::Partial(partial_standing_order) => {
                    if let Some(nonce) = self {
                        partial_standing_order.nonce = nonce;
                    }
                }
                StandingVariants::Exact(exact_standing_order) => {
                    if let Some(nonce) = self {
                        exact_standing_order.nonce = nonce;
                    }
                }
            }
        };

        Ok(())
    }
}

impl FillFrom<NonceGeneratorFiller, TransactionRequestWithLiquidityMeta> for Option<u64> {
    fn prepare_with(
        self,
        tx_req: &mut TransactionRequestWithLiquidityMeta,
    ) -> Result<(), FillerError> {
        tx_req.tx_request.nonce = self;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::{FixedBytes, address, aliases::I24};
    use alloy_provider::RootProvider;
    use alloy_rpc_types::TransactionRequest;
    use alloy_sol_types::SolCall;
    use angstrom_types::contract_bindings::pool_gate::PoolGate::addLiquidityCall;

    use crate::{
        AngstromApi, MakeFillerOrder,
        providers::AlloyRpcProvider,
        test_utils::{
            filler_orders::{AllOrdersSpecific, match_all_orders},
            spawn_angstrom_api,
        },
        types::fillers::AngstromFillProvider,
    };

    use super::*;

    async fn spawn_api_with_filler() -> eyre::Result<
        AngstromApi<AlloyRpcProvider<RootProvider>, AngstromFillProvider<(), NonceGeneratorFiller>>,
    > {
        Ok(spawn_angstrom_api().await?.with_nonce_generator_filler())
    }

    #[tokio::test]
    async fn test_nonce_generator_angstrom_order() {
        let api = spawn_api_with_filler().await.unwrap();
        let orders = AllOrdersSpecific::default();

        let provider = &api;
        orders
            .test_filler_order(async |order| {
                let mut inner_order = order.clone().convert_with_from(Address::default());

                provider
                    .filler
                    .fill(&provider.provider, &mut inner_order)
                    .await
                    .unwrap();

                let matched_orders =
                    match_all_orders(&inner_order.inner.force_all_orders(), &order, |o| match o {
                        AllOrders::Standing(StandingVariants::Exact(inner_order)) => {
                            Some(inner_order.nonce)
                        }
                        AllOrders::Standing(StandingVariants::Partial(inner_order)) => {
                            Some(inner_order.nonce)
                        }
                        _ => None,
                    });

                if let Some((mod_nonce, nonce)) = matched_orders {
                    nonce != mod_nonce
                } else {
                    true
                }
            })
            .await;
    }

    #[tokio::test]
    async fn test_nonce_generator_eth_order() {
        let api = spawn_api_with_filler().await.unwrap();

        let from = address!("0x429fd8e0040e2c982b2f91bf5ee75ce73015ec0c");
        let tx_req = TransactionRequest::default().from(from);

        let mut inner_order = TransactionRequestWithLiquidityMeta::new_add_liqudity(
            tx_req.clone(),
            addLiquidityCall::new((
                Address::default(),
                Address::default(),
                I24::default(),
                I24::default(),
                U256::default(),
                FixedBytes::<32>::default(),
            )),
        )
        .convert_with_from(from);
        api.filler
            .fill(&api.provider, &mut inner_order)
            .await
            .unwrap();

        assert!(tx_req.nonce.is_none());
        assert!(
            inner_order
                .inner
                .force_regular_tx()
                .tx_request
                .nonce
                .is_some()
        );
    }
}
