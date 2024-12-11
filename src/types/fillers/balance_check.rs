use crate::providers::EthProvider;
use alloy_primitives::{Address, U256};
use angstrom_types::primitive::ERC20;
use angstrom_types::sol_bindings::grouped_orders::AllOrders;
use angstrom_types::sol_bindings::grouped_orders::{FlashVariants, StandingVariants};

use super::AngstromFiller;

#[derive(Clone, Copy, Debug, Default)]
pub struct TokenBalanceCheckFiller {
    my_address: Address,
}

impl TokenBalanceCheckFiller {
    pub fn new(my_address: Address) -> Self {
        Self { my_address }
    }
}

impl AngstromFiller<AllOrders> for TokenBalanceCheckFiller {
    type FillOutput = ();

    async fn prepare<E: EthProvider>(
        &self,
        provider: &E,
        order: &AllOrders,
    ) -> eyre::Result<Self::FillOutput> {
        let (token, amt) = token_amount_from_all_orders(order);

        let user_balance_of = provider
            .view_call(
                token,
                ERC20::balanceOfCall {
                    _owner: self.my_address,
                },
            )
            .await?
            .balance;

        if amt < user_balance_of {
            return Err(eyre::eyre!("balance of {token:?} too low. in order: {amt:?} - current balance: {user_balance_of:?}"));
        }

        Ok(())
    }
}

fn token_amount_from_all_orders(order: &AllOrders) -> (Address, U256) {
    let (token, amt) = match order {
        AllOrders::Standing(standing_variants) => match standing_variants {
            StandingVariants::Partial(partial_standing_order) => (
                partial_standing_order.asset_in,
                partial_standing_order.max_amount_in + partial_standing_order.max_extra_fee_asset0,
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

    (token, U256::from(amt))
}
