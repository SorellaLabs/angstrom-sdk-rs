use std::fmt::Debug;

use alloy_primitives::{Address, B256, U256, hex, keccak256};
use alloy_provider::Provider;
use angstrom_types_primitives::{
    primitive::ANGSTROM_ADDRESS,
    sol_bindings::{RawPoolOrder, grouped_orders::AllOrders}
};

use super::{FillFrom, FillWrapper, errors::FillerError};
use crate::{apis::node_api::AngstromOrderApiClient, providers::backend::AngstromProvider};

/// The nonce location for quick db lookup
const ANGSTROM_NONCE_SLOT_CONST: [u8; 4] = hex!("daa050e9");

fn get_nonce_word_slot(user: Address, nonce: u64) -> B256 {
    let nonce = nonce.to_be_bytes();
    let mut arry = [0u8; 31];
    arry[0..20].copy_from_slice(&**user);
    arry[20..24].copy_from_slice(&ANGSTROM_NONCE_SLOT_CONST);
    arry[24..31].copy_from_slice(&nonce[0..7]);
    keccak256(arry)
}
#[derive(Clone, Copy, Debug, Default)]
pub struct NonceGeneratorFiller;

impl NonceGeneratorFiller {
    async fn get_valid_angstrom_nonce<P: Provider>(
        user: Address,
        provider: &P
    ) -> Result<u64, FillerError> {
        let mut nonce: u64 = rand::random();
        loop {
            let slot = get_nonce_word_slot(user, nonce);

            let word = provider
                .get_storage_at(*ANGSTROM_ADDRESS.get().unwrap(), slot.into())
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

#[async_trait::async_trait]
impl FillWrapper for NonceGeneratorFiller {
    type FillOutput = Option<u64>;

    async fn prepare<P, T>(
        &self,
        provider: &AngstromProvider<P, T>,
        order: &AllOrders
    ) -> Result<Self::FillOutput, FillerError>
    where
        P: Provider,
        T: AngstromOrderApiClient
    {
        if !matches!(order, AllOrders::PartialStanding(_) | AllOrders::ExactStanding(_)) {
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
        match input_order {
            AllOrders::ExactStanding(ex) => {
                if let Some(nonce) = self {
                    ex.nonce = nonce;
                }
            }
            AllOrders::PartialStanding(ex) => {
                if let Some(nonce) = self {
                    ex.nonce = nonce;
                }
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy_provider::RootProvider;
    use jsonrpsee_http_client::HttpClient;

    use super::*;
    use crate::{
        AngstromApi,
        test_utils::{
            AlloyRpcProvider,
            filler_orders::{AllOrdersSpecific, match_all_orders},
            spawn_angstrom_api
        },
        types::fillers::AngstromFillProvider
    };

    async fn spawn_api_with_filler() -> eyre::Result<
        AngstromApi<
            AlloyRpcProvider<RootProvider>,
            HttpClient,
            AngstromFillProvider<(), NonceGeneratorFiller>
        >
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
                    AllOrders::ExactStanding(inner_order) => Some(inner_order.nonce),
                    AllOrders::PartialStanding(inner_order) => Some(inner_order.nonce),
                    _ => None
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
