use alloy_primitives::{Address, U256};
use angstrom_types_primitives::primitive::ERC20;

use super::{AllOrders, FillWrapper, errors::FillerError};
use crate::{
    l1::{apis::node_api::AngstromOrderApiClient, providers::backend::AngstromProvider},
    types::{common::*, providers::alloy_view_call}
};

#[derive(Clone, Copy, Debug, Default)]
pub struct TokenBalanceCheckFiller;

impl TokenBalanceCheckFiller {
    async fn check_balance<T: AngstromOrderApiClient>(
        provider: &AngstromProvider<T>,
        user: Address,
        token: Address,
        requested_amount: U256
    ) -> Result<(), FillerError> {
        let user_balance_of = alloy_view_call(
            provider.eth_provider(),
            None,
            token,
            ERC20::balanceOfCall { _owner: user }
        )
        .await??;

        if requested_amount > user_balance_of {
            return Err(FillerError::InsufficientBalanceError(
                token,
                requested_amount,
                user_balance_of
            ));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl FillWrapper for TokenBalanceCheckFiller {
    type FillOutput = ();

    async fn prepare<T: AngstromOrderApiClient>(
        &self,
        provider: &AngstromProvider<T>,
        order: &AllOrders
    ) -> Result<Self::FillOutput, FillerError> {
        if order.from_address() != Address::ZERO {
            let (token, amt) = match order {
                AllOrders::PartialStanding(partial_standing_order) => (
                    partial_standing_order.asset_in,
                    partial_standing_order.max_amount_in
                        + partial_standing_order.max_extra_fee_asset0
                ),
                AllOrders::ExactStanding(exact_standing_order) => (
                    exact_standing_order.asset_in,
                    exact_standing_order.amount + exact_standing_order.max_extra_fee_asset0
                ),
                AllOrders::ExactFlash(exact_flash_order) => (
                    exact_flash_order.asset_in,
                    exact_flash_order.amount + exact_flash_order.max_extra_fee_asset0
                ),
                AllOrders::PartialFlash(partial_flash_order) => (
                    partial_flash_order.asset_in,
                    partial_flash_order.max_amount_in + partial_flash_order.max_extra_fee_asset0
                ),

                AllOrders::TOB(top_of_block_order) => (
                    top_of_block_order.asset_in,
                    top_of_block_order.quantity_in + top_of_block_order.max_gas_asset0
                )
            };

            Self::check_balance(provider, order.from_address(), token, U256::from(amt)).await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use alloy_signer_local::LocalSigner;

    use super::*;
    use crate::l1::{
        AngstromApi,
        test_utils::{
            USDC,
            filler_orders::{AllOrdersSpecific, AnvilAngstromProvider}
        }
    };

    #[tokio::test(flavor = "multi_thread")]
    async fn test_balance_checker_angstrom_order() {
        let signer = LocalSigner::random();
        let from = signer.address();

        let provider = AnvilAngstromProvider::new().await.unwrap();
        let api = AngstromApi::new_with_provider(provider.provider.clone())
            .with_token_balance_filler()
            .with_angstrom_signer_filler(signer);

        let orders = AllOrdersSpecific::default();

        let amount = 1000000000000000;
        let max_fee = 1000000000;
        let asset = USDC;

        let ref_api = &api;
        orders
            .clone()
            .test_filler_order(async |mut order| {
                match &mut order {
                    AllOrders::ExactStanding(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.amount = amount;
                        inner_order.meta.from = from;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::PartialStanding(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.max_amount_in = amount;
                        inner_order.meta.from = from;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::ExactFlash(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.amount = amount;
                        inner_order.meta.from = from;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::PartialFlash(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.max_amount_in = amount;
                        inner_order.meta.from = from;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::TOB(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.quantity_in = amount;
                        inner_order.meta.from = from;
                        inner_order.max_gas_asset0 = max_fee;
                    }
                }

                let mut inner_order = order.clone();

                let fill = ref_api.fill(&mut inner_order).await;

                matches!(fill.err().unwrap(), FillerError::InsufficientBalanceError(_, _, _))
            })
            .await;

        provider.overwrite_token_amounts(from, asset).await.unwrap();

        let api =
            AngstromApi::new_with_provider(provider.provider.clone()).with_token_balance_filler();
        let ref_api = &api;
        orders
            .test_filler_order(async |mut order| {
                match &mut order {
                    AllOrders::ExactStanding(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.amount = amount;
                        inner_order.meta.from = from;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::PartialStanding(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.max_amount_in = amount;
                        inner_order.meta.from = from;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::ExactFlash(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.amount = amount;
                        inner_order.meta.from = from;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::PartialFlash(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.max_amount_in = amount;
                        inner_order.meta.from = from;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::TOB(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.quantity_in = amount;
                        inner_order.meta.from = from;
                        inner_order.max_gas_asset0 = max_fee;
                    }
                }

                let mut inner_order = order.clone();

                ref_api.fill(&mut inner_order).await.is_ok()
            })
            .await;
    }
}
