use super::{AngstromFiller, FillFrom, errors::FillerError};
use crate::{providers::backend::AngstromProvider, types::ANGSTROM_ADDRESS};
use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use angstrom_types::sol_bindings::{
    RawPoolOrder,
    grouped_orders::{AllOrders, StandingVariants},
};

use validation::order::state::db_state_utils::nonces::Nonces;

#[derive(Clone, Copy, Debug, Default)]
pub struct NonceGeneratorFiller;

impl NonceGeneratorFiller {
    async fn get_valid_angstrom_nonce<P: Provider>(
        user: Address,
        provider: &P,
    ) -> Result<u64, FillerError> {
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
}

impl AngstromFiller for NonceGeneratorFiller {
    type FillOutput = Option<u64>;

    async fn prepare<P>(
        &self,
        provider: &AngstromProvider<P>,
        order: &AllOrders,
    ) -> Result<Self::FillOutput, FillerError>
    where
        P: Provider,
    {
        if !matches!(order, AllOrders::Standing(_)) {
            return Ok(None);
        }

        if order.from() != Address::default() {
            let nonce =
                Self::get_valid_angstrom_nonce(order.from(), provider.eth_provider()).await?;
            Ok(Some(nonce))
        } else {
            Ok(None)
        }
    }
}

impl FillFrom<NonceGeneratorFiller> for Option<u64> {
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

#[cfg(test)]
mod tests {
    use alloy_provider::RootProvider;

    use crate::{
        AngstromApi,
        providers::backend::AlloyRpcProvider,
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
            .test_filler_order(async |order1| {
                let mut order0 = order1.clone();

                provider.fill(&mut order0).await.unwrap();

                let matched_orders = match_all_orders(&order0, &order1, |o| match o {
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
}
