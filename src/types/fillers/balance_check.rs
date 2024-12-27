use alloy_primitives::U256;
use alloy_provider::Provider;
use alloy_transport::Transport;
use angstrom_types::{
    primitive::ERC20,
    sol_bindings::{
        grouped_orders::{AllOrders, FlashVariants, StandingVariants},
        RawPoolOrder
    }
};

use super::{AngstromFiller, FillerOrder};
use crate::{
    providers::{AngstromProvider, EthRpcProvider},
    types::TransactionRequestWithLiquidityMeta
};

#[derive(Clone, Copy, Debug, Default)]
pub struct TokenBalanceCheckFiller;

impl TokenBalanceCheckFiller {
    async fn handle_angstrom_order<P, T>(
        &self,
        provider: &EthRpcProvider<P, T>,
        order: &AllOrders
    ) -> eyre::Result<()>
    where
        P: Provider<T> + Clone,
        T: Transport + Clone
    {
        let (token, amt) = match order {
            AllOrders::Standing(standing_variants) => match standing_variants {
                StandingVariants::Partial(partial_standing_order) => (
                    partial_standing_order.asset_in,
                    partial_standing_order.max_amount_in
                        + partial_standing_order.max_extra_fee_asset0
                ),
                StandingVariants::Exact(exact_standing_order) => (
                    exact_standing_order.asset_in,
                    exact_standing_order.amount + exact_standing_order.max_extra_fee_asset0
                )
            },
            AllOrders::Flash(flash_variants) => match flash_variants {
                FlashVariants::Partial(partial_flash_order) => (
                    partial_flash_order.asset_in,
                    partial_flash_order.max_amount_in + partial_flash_order.max_extra_fee_asset0
                ),
                FlashVariants::Exact(exact_flash_order) => (
                    exact_flash_order.asset_in,
                    exact_flash_order.amount + exact_flash_order.max_extra_fee_asset0
                )
            },
            AllOrders::TOB(top_of_block_order) => (
                top_of_block_order.asset_in,
                top_of_block_order.quantity_in + top_of_block_order.max_gas_asset0
            )
        };

        let user_balance_of = provider
            .view_call(token, ERC20::balanceOfCall { _owner: order.from() })
            .await?
            .balance;

        if U256::from(amt) < user_balance_of {
            return Err(eyre::eyre!(
                "balance of {token:?} too low. in order: {amt} - current balance: \
                 {user_balance_of:?}"
            ));
        }

        Ok(())
    }

    async fn handle_liquidity_order<P, T>(
        &self,
        provider: &EthRpcProvider<P, T>,
        order: &TransactionRequestWithLiquidityMeta
    ) -> eyre::Result<()>
    where
        P: Provider<T> + Clone,
        T: Transport + Clone
    {
        // todo!();
        Ok(())
    }
}

impl AngstromFiller for TokenBalanceCheckFiller {
    type FillOutput = ();

    async fn prepare<P, T>(
        &self,
        provider: &EthRpcProvider<P, T>,
        _: &AngstromProvider,
        order: &FillerOrder
    ) -> eyre::Result<Self::FillOutput>
    where
        P: Provider<T> + Clone,
        T: Transport + Clone
    {
        match order {
            FillerOrder::AngstromOrder(angstrom_order) => {
                self.handle_angstrom_order(provider, angstrom_order).await
            }
            FillerOrder::RegularOrder(liquidity_order) => {
                self.handle_liquidity_order(provider, liquidity_order).await
            }
        }
    }
}
