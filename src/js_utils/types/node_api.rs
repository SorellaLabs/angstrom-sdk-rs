use std::collections::HashMap;

use alloy_primitives::{
    aliases::{I24, U24, U40},
    Address, Bytes, B256, U256
};
use angstrom_sdk_macros::{neon_object_as, NeonObject};
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey,
    contract_payloads::{
        angstrom::{OrderQuantities, StandingValidation, UserOrder},
        Signature
    },
    primitive::{PoolId, UniswapPoolRegistry},
    sol_bindings::{
        grouped_orders::{AllOrders, FlashVariants, StandingVariants},
        rpc_orders::{
            ExactFlashOrder, ExactStandingOrder, OrderMeta, PartialFlashOrder,
            PartialStandingOrder, TopOfBlockOrder
        }
    }
};
use neon::{context::Context, object::Object};
use uniswap_v4::uniswap::{
    pool::{EnhancedUniswapPool, TickInfo},
    pool_data_loader::{DataLoader, PoolDataLoader}
};
use validation::order::OrderPoolNewOrderResult;

use crate::types::HistoricalOrders;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub enum AllOrdersNeon {
    Standing { order: StandingVariantsNeon },
    Flash { order: FlashVariantsNeon },
    TOB { order: TopOfBlockOrderSolBindingsNeon }
}

impl From<AllOrders> for AllOrdersNeon {
    fn from(value: AllOrders) -> Self {
        match value {
            AllOrders::Standing(standing_variants) => {
                AllOrdersNeon::Standing { order: standing_variants.into() }
            }
            AllOrders::Flash(flash_variants) => {
                AllOrdersNeon::Flash { order: flash_variants.into() }
            }
            AllOrders::TOB(top_of_block_order) => {
                AllOrdersNeon::TOB { order: top_of_block_order.into() }
            }
        }
    }
}

impl Into<AllOrders> for AllOrdersNeon {
    fn into(self) -> AllOrders {
        match self {
            Self::Standing { order } => AllOrders::Standing(order.into()),
            Self::Flash { order } => AllOrders::Flash(order.into()),
            Self::TOB { order } => AllOrders::TOB(order.into())
        }
    }
}

neon_object_as!(AllOrders, AllOrdersNeon);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
enum StandingVariantsNeon {
    Partial { order: PartialStandingOrderNeon },
    Exact { order: ExactStandingOrderNeon }
}

impl From<StandingVariants> for StandingVariantsNeon {
    fn from(value: StandingVariants) -> Self {
        match value {
            StandingVariants::Partial(order) => Self::Partial { order: order.into() },
            StandingVariants::Exact(order) => Self::Exact { order: order.into() }
        }
    }
}

impl Into<StandingVariants> for StandingVariantsNeon {
    fn into(self) -> StandingVariants {
        match self {
            Self::Partial { order } => StandingVariants::Partial(order.into()),
            Self::Exact { order } => StandingVariants::Exact(order.into())
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
struct PartialStandingOrderNeon {
    ref_id:               u32,
    min_amount_in:        u128,
    max_amount_in:        u128,
    max_extra_fee_asset0: u128,
    min_price:            U256,
    use_internal:         bool,
    asset_in:             Address,
    asset_out:            Address,
    recipient:            Address,
    hook_data:            Bytes,
    nonce:                u64,
    deadline:             u64,
    meta:                 OrderMetaNeon
}

impl From<PartialStandingOrder> for PartialStandingOrderNeon {
    fn from(value: PartialStandingOrder) -> Self {
        Self {
            ref_id:               value.ref_id,
            min_amount_in:        value.min_amount_in,
            max_amount_in:        value.max_amount_in,
            max_extra_fee_asset0: value.max_extra_fee_asset0,
            min_price:            value.min_price,
            use_internal:         value.use_internal,
            asset_in:             value.asset_in,
            asset_out:            value.asset_out,
            recipient:            value.recipient,
            hook_data:            value.hook_data,
            nonce:                value.nonce,
            deadline:             value.deadline.to(),
            meta:                 value.meta.into()
        }
    }
}

impl Into<PartialStandingOrder> for PartialStandingOrderNeon {
    fn into(self) -> PartialStandingOrder {
        PartialStandingOrder {
            ref_id:               self.ref_id,
            min_amount_in:        self.min_amount_in,
            max_amount_in:        self.max_amount_in,
            max_extra_fee_asset0: self.max_extra_fee_asset0,
            min_price:            self.min_price,
            use_internal:         self.use_internal,
            asset_in:             self.asset_in,
            asset_out:            self.asset_out,
            recipient:            self.recipient,
            hook_data:            self.hook_data,
            nonce:                self.nonce,
            deadline:             U40::from(self.deadline),
            meta:                 self.meta.into()
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
struct ExactStandingOrderNeon {
    ref_id:               u32,
    exact_in:             bool,
    amount:               u128,
    max_extra_fee_asset0: u128,
    min_price:            U256,
    use_internal:         bool,
    asset_in:             Address,
    asset_out:            Address,
    recipient:            Address,
    hook_data:            Bytes,
    nonce:                u64,
    deadline:             u64,
    meta:                 OrderMetaNeon
}

impl From<ExactStandingOrder> for ExactStandingOrderNeon {
    fn from(value: ExactStandingOrder) -> Self {
        Self {
            ref_id:               value.ref_id,
            max_extra_fee_asset0: value.max_extra_fee_asset0,
            min_price:            value.min_price,
            use_internal:         value.use_internal,
            asset_in:             value.asset_in,
            asset_out:            value.asset_out,
            recipient:            value.recipient,
            hook_data:            value.hook_data,
            nonce:                value.nonce,
            deadline:             value.deadline.to(),
            meta:                 value.meta.into(),
            exact_in:             value.exact_in,
            amount:               value.amount
        }
    }
}

impl Into<ExactStandingOrder> for ExactStandingOrderNeon {
    fn into(self) -> ExactStandingOrder {
        ExactStandingOrder {
            ref_id:               self.ref_id,
            max_extra_fee_asset0: self.max_extra_fee_asset0,
            min_price:            self.min_price,
            use_internal:         self.use_internal,
            asset_in:             self.asset_in,
            asset_out:            self.asset_out,
            recipient:            self.recipient,
            hook_data:            self.hook_data,
            nonce:                self.nonce,
            deadline:             U40::from(self.deadline),
            meta:                 self.meta.into(),
            exact_in:             self.exact_in,
            amount:               self.amount
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
enum FlashVariantsNeon {
    Partial { order: PartialFlashOrderNeon },
    Exact { order: ExactFlashOrderNeon }
}

impl From<FlashVariants> for FlashVariantsNeon {
    fn from(value: FlashVariants) -> Self {
        match value {
            FlashVariants::Partial(order) => Self::Partial { order: order.into() },
            FlashVariants::Exact(order) => Self::Exact { order: order.into() }
        }
    }
}

impl Into<FlashVariants> for FlashVariantsNeon {
    fn into(self) -> FlashVariants {
        match self {
            Self::Partial { order } => FlashVariants::Partial(order.into()),
            Self::Exact { order } => FlashVariants::Exact(order.into())
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
struct PartialFlashOrderNeon {
    ref_id:               u32,
    min_amount_in:        u128,
    max_amount_in:        u128,
    max_extra_fee_asset0: u128,
    min_price:            U256,
    use_internal:         bool,
    asset_in:             Address,
    asset_out:            Address,
    recipient:            Address,
    hook_data:            Bytes,
    valid_for_block:      u64,
    meta:                 OrderMetaNeon
}

impl From<PartialFlashOrder> for PartialFlashOrderNeon {
    fn from(value: PartialFlashOrder) -> Self {
        Self {
            ref_id:               value.ref_id,
            min_amount_in:        value.min_amount_in,
            max_amount_in:        value.max_amount_in,
            max_extra_fee_asset0: value.max_extra_fee_asset0,
            min_price:            value.min_price,
            use_internal:         value.use_internal,
            asset_in:             value.asset_in,
            asset_out:            value.asset_out,
            recipient:            value.recipient,
            hook_data:            value.hook_data,
            valid_for_block:      value.valid_for_block,
            meta:                 value.meta.into()
        }
    }
}

impl Into<PartialFlashOrder> for PartialFlashOrderNeon {
    fn into(self) -> PartialFlashOrder {
        PartialFlashOrder {
            ref_id:               self.ref_id,
            min_amount_in:        self.min_amount_in,
            max_amount_in:        self.max_amount_in,
            max_extra_fee_asset0: self.max_extra_fee_asset0,
            min_price:            self.min_price,
            use_internal:         self.use_internal,
            asset_in:             self.asset_in,
            asset_out:            self.asset_out,
            recipient:            self.recipient,
            hook_data:            self.hook_data,
            valid_for_block:      self.valid_for_block,
            meta:                 self.meta.into()
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
struct ExactFlashOrderNeon {
    ref_id:               u32,
    exact_in:             bool,
    amount:               u128,
    max_extra_fee_asset0: u128,
    min_price:            U256,
    use_internal:         bool,
    asset_in:             Address,
    asset_out:            Address,
    recipient:            Address,
    hook_data:            Bytes,
    valid_for_block:      u64,
    meta:                 OrderMetaNeon
}

impl From<ExactFlashOrder> for ExactFlashOrderNeon {
    fn from(value: ExactFlashOrder) -> Self {
        Self {
            ref_id:               value.ref_id,
            max_extra_fee_asset0: value.max_extra_fee_asset0,
            min_price:            value.min_price,
            use_internal:         value.use_internal,
            asset_in:             value.asset_in,
            asset_out:            value.asset_out,
            recipient:            value.recipient,
            hook_data:            value.hook_data,
            valid_for_block:      value.valid_for_block,
            meta:                 value.meta.into(),
            exact_in:             value.exact_in,
            amount:               value.amount
        }
    }
}

impl Into<ExactFlashOrder> for ExactFlashOrderNeon {
    fn into(self) -> ExactFlashOrder {
        ExactFlashOrder {
            ref_id:               self.ref_id,
            max_extra_fee_asset0: self.max_extra_fee_asset0,
            min_price:            self.min_price,
            use_internal:         self.use_internal,
            asset_in:             self.asset_in,
            asset_out:            self.asset_out,
            recipient:            self.recipient,
            hook_data:            self.hook_data,
            valid_for_block:      self.valid_for_block,
            meta:                 self.meta.into(),
            exact_in:             self.exact_in,
            amount:               self.amount
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
struct TopOfBlockOrderSolBindingsNeon {
    quantity_in:     u128,
    quantity_out:    u128,
    max_gas_asset0:  u128,
    use_internal:    bool,
    asset_in:        Address,
    asset_out:       Address,
    recipient:       Address,
    valid_for_block: u64,
    meta:            OrderMetaNeon
}

impl From<TopOfBlockOrder> for TopOfBlockOrderSolBindingsNeon {
    fn from(value: TopOfBlockOrder) -> Self {
        Self {
            quantity_in:     value.quantity_in,
            quantity_out:    value.quantity_out,
            max_gas_asset0:  value.max_gas_asset0,
            use_internal:    value.use_internal,
            asset_in:        value.asset_in,
            asset_out:       value.asset_out,
            recipient:       value.recipient,
            valid_for_block: value.valid_for_block,
            meta:            value.meta.into()
        }
    }
}

impl Into<TopOfBlockOrder> for TopOfBlockOrderSolBindingsNeon {
    fn into(self) -> TopOfBlockOrder {
        TopOfBlockOrder {
            quantity_in:     self.quantity_in,
            quantity_out:    self.quantity_out,
            max_gas_asset0:  self.max_gas_asset0,
            use_internal:    self.use_internal,
            asset_in:        self.asset_in,
            asset_out:       self.asset_out,
            recipient:       self.recipient,
            valid_for_block: self.valid_for_block,
            meta:            self.meta.into()
        }
    }
}

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
struct OrderMetaNeon {
    isEcdsa:   bool,
    from:      Address,
    signature: Bytes
}

impl From<OrderMeta> for OrderMetaNeon {
    fn from(value: OrderMeta) -> Self {
        Self { isEcdsa: value.isEcdsa, from: value.from, signature: value.signature }
    }
}

impl Into<OrderMeta> for OrderMetaNeon {
    fn into(self) -> OrderMeta {
        OrderMeta { isEcdsa: self.isEcdsa, from: self.from, signature: self.signature }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub enum OrderPoolNewOrderResultNeon {
    Valid,
    Invalid,
    TransitionedToBlock,
    Error { error: String }
}

impl From<OrderPoolNewOrderResult> for OrderPoolNewOrderResultNeon {
    fn from(value: OrderPoolNewOrderResult) -> Self {
        match value {
            OrderPoolNewOrderResult::Valid => Self::Valid,
            OrderPoolNewOrderResult::Invalid => Self::Invalid,
            OrderPoolNewOrderResult::TransitionedToBlock => Self::TransitionedToBlock,
            OrderPoolNewOrderResult::Error(error) => Self::Error { error }
        }
    }
}

impl Into<OrderPoolNewOrderResult> for OrderPoolNewOrderResultNeon {
    fn into(self) -> OrderPoolNewOrderResult {
        match self {
            OrderPoolNewOrderResultNeon::Valid => OrderPoolNewOrderResult::Valid,
            OrderPoolNewOrderResultNeon::Invalid => OrderPoolNewOrderResult::Invalid,
            OrderPoolNewOrderResultNeon::TransitionedToBlock => {
                OrderPoolNewOrderResult::TransitionedToBlock
            }
            OrderPoolNewOrderResultNeon::Error { error } => OrderPoolNewOrderResult::Error(error)
        }
    }
}

neon_object_as!(OrderPoolNewOrderResult, OrderPoolNewOrderResultNeon);
