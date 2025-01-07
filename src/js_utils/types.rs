use alloy_primitives::{
    aliases::{I24, U24},
    Address, Bytes, B256
};
use angstrom_sdk_macros::{neon_object_as, NeonObject};
use angstrom_types::{
    contract_bindings::angstrom::Angstrom::PoolKey,
    contract_payloads::{angstrom::TopOfBlockOrder, Signature}
};
use neon::{context::Context, object::Object};

use super::MakeObject;

#[allow(non_snake_case)]
#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub struct PoolKeyNeon {
    pub currency0:   Address,
    pub currency1:   Address,
    pub fee:         U24,
    pub tickSpacing: I24,
    pub hooks:       Address
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
pub struct TopOfBlockOrderNeon {
    pub use_internal:     bool,
    pub quantity_in:      u128,
    pub quantity_out:     u128,
    pub max_gas_asset_0:  u128,
    pub gas_used_asset_0: u128,
    pub pairs_index:      u16,
    pub zero_for_1:       bool,
    pub recipient:        Option<Address>,
    pub signature:        SignatureNeon
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

#[derive(Debug, Clone)]
#[cfg_attr(feature = "neon", derive(NeonObject))]
pub enum SignatureNeon {
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
