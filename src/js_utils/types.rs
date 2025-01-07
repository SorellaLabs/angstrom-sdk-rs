use alloy_primitives::{
    aliases::{I24, U24},
    Address, Bytes, B256, U256
};
use angstrom_sdk_macros::{neon_object_as, NeonObject};
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey,
    contract_payloads::{
        angstrom::{OrderQuantities, StandingValidation, TopOfBlockOrder, UserOrder},
        Signature
    }
};
use neon::{context::Context, object::Object};

use crate::types::HistoricalOrders;

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub struct PoolKeyNeon {
    currency0:   Address,
    currency1:   Address,
    fee:         U24,
    tickSpacing: I24,
    hooks:       Address
}

impl From<PoolKey> for PoolKeyNeon {
    fn from(value: PoolKey) -> Self {
        Self {
            currency0:   value.currency0,
            currency1:   value.currency1,
            fee:         value.fee,
            tickSpacing: value.tickSpacing,
            hooks:       value.hooks
        }
    }
}

neon_object_as!(PoolKey, PoolKeyNeon);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub enum HistoricalOrdersNeon {
    TOB { order: TopOfBlockOrderNeon },
    User { order: UserOrderNeon }
}

impl From<HistoricalOrders> for HistoricalOrdersNeon {
    fn from(value: HistoricalOrders) -> Self {
        match value {
            HistoricalOrders::TOB(top_of_block_order) => {
                Self::TOB { order: top_of_block_order.into() }
            }
            HistoricalOrders::User(user_order) => Self::User { order: user_order.into() }
        }
    }
}

neon_object_as!(HistoricalOrders, HistoricalOrdersNeon);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub struct TopOfBlockOrderNeon {
    use_internal:     bool,
    quantity_in:      u128,
    quantity_out:     u128,
    max_gas_asset_0:  u128,
    gas_used_asset_0: u128,
    pairs_index:      u16,
    zero_for_1:       bool,
    recipient:        Option<Address>,
    signature:        SignatureNeon
}

impl From<TopOfBlockOrder> for TopOfBlockOrderNeon {
    fn from(value: TopOfBlockOrder) -> Self {
        Self {
            use_internal:     value.use_internal,
            quantity_in:      value.quantity_in,
            quantity_out:     value.quantity_out,
            max_gas_asset_0:  value.max_gas_asset_0,
            gas_used_asset_0: value.gas_used_asset_0,
            pairs_index:      value.pairs_index,
            zero_for_1:       value.zero_for_1,
            recipient:        value.recipient,
            signature:        value.signature.into()
        }
    }
}

neon_object_as!(TopOfBlockOrder, TopOfBlockOrderNeon);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub struct UserOrderNeon {
    ref_id:               u32,
    use_internal:         bool,
    pair_index:           u16,
    min_price:            U256,
    recipient:            Option<Address>,
    hook_data:            Option<Bytes>,
    zero_for_one:         bool,
    standing_validation:  Option<StandingValidationNeon>,
    order_quantities:     OrderQuantitiesNeon,
    max_extra_fee_asset0: u128,
    extra_fee_asset0:     u128,
    exact_in:             bool,
    signature:            SignatureNeon
}

impl From<UserOrder> for UserOrderNeon {
    fn from(value: UserOrder) -> Self {
        Self {
            ref_id:               value.ref_id,
            recipient:            value.recipient,
            signature:            value.signature.into(),
            use_internal:         value.use_internal,
            pair_index:           value.pair_index,
            min_price:            value.min_price,
            hook_data:            value.hook_data,
            zero_for_one:         value.zero_for_one,
            standing_validation:  value.standing_validation.map(Into::into),
            order_quantities:     value.order_quantities.into(),
            max_extra_fee_asset0: value.max_extra_fee_asset0,
            extra_fee_asset0:     value.extra_fee_asset0,
            exact_in:             value.exact_in
        }
    }
}

neon_object_as!(UserOrder, UserOrderNeon);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
enum SignatureNeon {
    Contract { from: Address, signature: Bytes },
    Ecdsa { v: u8, r: B256, s: B256 }
}

impl From<Signature> for SignatureNeon {
    fn from(value: Signature) -> Self {
        match value {
            Signature::Contract { from, signature } => Self::Contract { from, signature },
            Signature::Ecdsa { v, r, s } => Self::Ecdsa { v, r, s }
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
enum OrderQuantitiesNeon {
    Exact { quantity: u128 },
    Partial { min_quantity_in: u128, max_quantity_in: u128, filled_quantity: u128 }
}

impl From<OrderQuantities> for OrderQuantitiesNeon {
    fn from(value: OrderQuantities) -> Self {
        match value {
            OrderQuantities::Exact { quantity } => Self::Exact { quantity },
            OrderQuantities::Partial { min_quantity_in, max_quantity_in, filled_quantity } => {
                Self::Partial { min_quantity_in, max_quantity_in, filled_quantity }
            }
        }
    }
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
struct StandingValidationNeon {
    nonce:    u64,
    deadline: u64
}

impl From<StandingValidation> for StandingValidationNeon {
    fn from(value: StandingValidation) -> Self {
        StandingValidationNeon { nonce: value.nonce(), deadline: value.deadline() }
    }
}
