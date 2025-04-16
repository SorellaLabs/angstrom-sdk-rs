use alloy_primitives::{Address, U256};
use alloy_provider::Provider;
use angstrom_types::{
    primitive::ERC20,
    sol_bindings::grouped_orders::{AllOrders, FlashVariants, StandingVariants},
};

use super::{AngstromFiller, FillerOrder, FillerOrderFrom, errors::FillerError};
use crate::{providers::backend::AngstromProvider, types::TransactionRequestWithLiquidityMeta};

#[derive(Clone, Copy, Debug, Default)]
pub struct TokenBalanceCheckFiller;

impl TokenBalanceCheckFiller {
    async fn prepare_angstrom_order<P>(
        &self,
        provider: &AngstromProvider<P>,
        order: &AllOrders,
        from: Address,
    ) -> Result<(), FillerError>
    where
        P: Provider,
    {
        let (token, amt) = match order {
            AllOrders::Standing(standing_variants) => match standing_variants {
                StandingVariants::Partial(partial_standing_order) => (
                    partial_standing_order.asset_in,
                    partial_standing_order.max_amount_in
                        + partial_standing_order.max_extra_fee_asset0,
                ),
                StandingVariants::Exact(exact_standing_order) => (
                    exact_standing_order.asset_in,
                    exact_standing_order.amount + exact_standing_order.max_extra_fee_asset0,
                ),
            },
            AllOrders::Flash(flash_variants) => match flash_variants {
                FlashVariants::Partial(partial_flash_order) => (
                    partial_flash_order.asset_in,
                    partial_flash_order.max_amount_in + partial_flash_order.max_extra_fee_asset0,
                ),
                FlashVariants::Exact(exact_flash_order) => (
                    exact_flash_order.asset_in,
                    exact_flash_order.amount + exact_flash_order.max_extra_fee_asset0,
                ),
            },
            AllOrders::TOB(top_of_block_order) => (
                top_of_block_order.asset_in,
                top_of_block_order.quantity_in + top_of_block_order.max_gas_asset0,
            ),
        };

        let user_balance_of = provider
            .view_call(token, ERC20::balanceOfCall { _owner: from })
            .await??
            .balance;

        let amt_u256 = U256::from(amt);
        if U256::from(amt_u256) > user_balance_of {
            return Err(FillerError::InsufficientBalanceError(token, amt_u256, user_balance_of));
        }

        Ok(())
    }

    async fn prepare_eth_order<P>(
        &self,
        _provider: &AngstromProvider<P>,
        _order: &TransactionRequestWithLiquidityMeta,
        _from: Address,
    ) -> Result<(), FillerError>
    where
        P: Provider,
    {
        // todo!();
        Ok(())
    }
}

impl AngstromFiller for TokenBalanceCheckFiller {
    type FillOutput = ();

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
            FillerOrder::EthOrder(inner_order) => {
                self.prepare_eth_order(provider, inner_order, order.from)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use alloy_primitives::Address;

    use crate::{
        AngstromApi,
        test_utils::filler_orders::{AllOrdersSpecific, AnvilAngstromProvider},
        types::{USDC, fillers::MakeFillerOrder},
    };

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_balance_checker_angstrom_order() {
        let provider = AnvilAngstromProvider::new().await.unwrap();
        let api =
            AngstromApi::new_with_provider(provider.provider.clone()).with_token_balance_filler();

        let orders = AllOrdersSpecific::default();

        let from = Address::random();
        let amount = 1000000000000000;
        let max_fee = 1000000000;
        let asset = USDC;

        let ref_api = &api;
        orders
            .clone()
            .test_filler_order(async |mut order| {
                match &mut order {
                    AllOrders::Standing(StandingVariants::Exact(inner_order)) => {
                        inner_order.asset_in = asset;
                        inner_order.amount = amount;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::Standing(StandingVariants::Partial(inner_order)) => {
                        inner_order.asset_in = asset;
                        inner_order.max_amount_in = amount;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::Flash(FlashVariants::Exact(inner_order)) => {
                        inner_order.asset_in = asset;
                        inner_order.amount = amount;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::Flash(FlashVariants::Partial(inner_order)) => {
                        inner_order.asset_in = asset;
                        inner_order.max_amount_in = amount;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::TOB(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.quantity_in = amount;
                        inner_order.max_gas_asset0 = max_fee;
                    }
                }

                let mut inner_order = order.clone().convert_with_from(from);

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
                    AllOrders::Standing(StandingVariants::Exact(inner_order)) => {
                        inner_order.asset_in = asset;
                        inner_order.amount = amount;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::Standing(StandingVariants::Partial(inner_order)) => {
                        inner_order.asset_in = asset;
                        inner_order.max_amount_in = amount;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::Flash(FlashVariants::Exact(inner_order)) => {
                        inner_order.asset_in = asset;
                        inner_order.amount = amount;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::Flash(FlashVariants::Partial(inner_order)) => {
                        inner_order.asset_in = asset;
                        inner_order.max_amount_in = amount;
                        inner_order.max_extra_fee_asset0 = max_fee;
                    }
                    AllOrders::TOB(inner_order) => {
                        inner_order.asset_in = asset;
                        inner_order.quantity_in = amount;
                        inner_order.max_gas_asset0 = max_fee;
                    }
                }

                let mut inner_order = order.clone().convert_with_from(from);

                ref_api.fill(&mut inner_order).await.is_ok()
            })
            .await;
    }
}
