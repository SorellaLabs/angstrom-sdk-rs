use crate::apis::node_api::AngstromNodeApi;
use alloy_primitives::{aliases::U40, Address, U256};
use alloy_signer::k256::ecdsa::{self, signature::hazmat::PrehashSigner, RecoveryId};
use alloy_signer_local::LocalSigner;
use angstrom_types::sol_bindings::{
    grouped_orders::{AllOrders, FlashVariants, StandingVariants},
    rpc_orders::{
        ExactFlashOrder, ExactStandingOrder, PartialFlashOrder, PartialStandingOrder,
        TopOfBlockOrder,
    },
};

use super::utils::sign_into_meta;

trait AngstromOrderSender<C>: AngstromNodeApi
where
    C: PrehashSigner<(ecdsa::Signature, RecoveryId)>,
{
    fn signer(&self) -> &LocalSigner<C>;

    async fn add_liquidity(
        &self,
        token0: Address,
        token1: Address,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: U256,
    );

    async fn remove_liquidity(
        &self,
        token0: Address,
        token1: Address,
        tick_lower: i32,
        tick_upper: i32,
        liquidity: U256,
    );

    async fn top_of_block_order(
        &self,
        asset_in: Address,
        asset_out: Address,
        quantity_in: u128,
        quantity_out: u128,
        max_gas_asset0: u128,
        valid_for_block: u64,
    ) -> eyre::Result<()> {
        let signer = self.signer();
        let mut order = TopOfBlockOrder {
            asset_in,
            asset_out,
            quantity_in,
            quantity_out,
            valid_for_block,
            recipient: signer.address(),
            max_gas_asset0,
            ..Default::default()
        };

        order.meta = sign_into_meta(signer, &order)?;

        self.send_order(AllOrders::TOB(order)).await?;

        Ok(())
    }

    async fn partial_standing_order(
        &self,
        asset_in: Address,
        asset_out: Address,
        min_amount_in: u128,
        max_amount_in: u128,
        min_price: U256,
        nonce: u64,
        max_extra_fee_asset0: Option<u128>,
        deadline: Option<u64>,
    ) -> eyre::Result<()> {
        let signer = self.signer();

        let mut order = PartialStandingOrder {
            asset_in,
            asset_out,
            max_amount_in,
            min_amount_in,
            min_price,
            recipient: signer.address(),
            nonce,
            max_extra_fee_asset0: max_extra_fee_asset0.unwrap_or_default(),
            deadline: deadline.map(|d| U40::from(d)).unwrap_or_default(),
            ..Default::default()
        };

        order.meta = sign_into_meta(signer, &order)?;

        self.send_order(AllOrders::Standing(StandingVariants::Partial(order)))
            .await?;

        Ok(())
    }

    async fn exact_standing_order(
        &self,
        asset_in: Address,
        asset_out: Address,
        exact_in: bool,
        amount: u128,
        min_price: U256,
        nonce: u64,
        max_extra_fee_asset0: Option<u128>,
        deadline: Option<u64>,
    ) -> eyre::Result<()> {
        let signer = self.signer();
        let mut order = ExactStandingOrder {
            asset_in,
            asset_out,
            min_price,
            recipient: signer.address(),
            nonce,
            max_extra_fee_asset0: max_extra_fee_asset0.unwrap_or_default(),
            deadline: deadline.map(|d| U40::from(d)).unwrap_or_default(),
            exact_in,
            amount,
            ..Default::default()
        };

        order.meta = sign_into_meta(signer, &order)?;

        self.send_order(AllOrders::Standing(StandingVariants::Exact(order)))
            .await?;

        Ok(())
    }

    async fn partial_flash_order(
        &self,
        asset_in: Address,
        asset_out: Address,
        min_amount_in: u128,
        max_amount_in: u128,
        min_price: U256,
        max_extra_fee_asset0: Option<u128>,
        valid_for_block: u64,
    ) -> eyre::Result<()> {
        let signer = self.signer();
        let mut order = PartialFlashOrder {
            asset_in,
            asset_out,
            max_amount_in,
            min_amount_in,
            min_price,
            recipient: signer.address(),
            max_extra_fee_asset0: max_extra_fee_asset0.unwrap_or_default(),
            valid_for_block,
            ..Default::default()
        };

        order.meta = sign_into_meta(signer, &order)?;

        self.send_order(AllOrders::Flash(FlashVariants::Partial(order)))
            .await?;

        Ok(())
    }

    async fn exact_flash_order(
        &self,
        asset_in: Address,
        asset_out: Address,
        exact_in: bool,
        amount: u128,
        min_price: U256,
        max_extra_fee_asset0: Option<u128>,
        valid_for_block: u64,
    ) -> eyre::Result<()> {
        let signer = self.signer();
        let mut order = ExactFlashOrder {
            asset_in,
            asset_out,
            min_price,
            recipient: signer.address(),
            max_extra_fee_asset0: max_extra_fee_asset0.unwrap_or_default(),
            exact_in,
            amount,
            valid_for_block,
            ..Default::default()
        };

        order.meta = sign_into_meta(signer, &order)?;

        self.send_order(AllOrders::Flash(FlashVariants::Exact(order)))
            .await?;

        Ok(())
    }
}
