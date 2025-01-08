use std::collections::HashMap;

use alloy_primitives::{
    aliases::{I24, U24},
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
