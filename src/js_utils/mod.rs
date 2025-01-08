use std::collections::HashMap;

use alloy_primitives::{
    aliases::{I24, U24},
    Address, Bytes, TxHash, B256, I256, U256
};
use angstrom_types::primitive::PoolId;
use neon::{
    object::Object,
    prelude::{Context, FunctionContext, Handle, TaskContext},
    result::{NeonResult, Throw},
    types::{
        JsArray, JsBigInt, JsBoolean, JsNull, JsNumber, JsObject, JsString, JsUint8Array, JsValue,
        Value
    }
};

mod types;

pub trait MakeObject<S = Self>
where
    S: From<Self> + Clone,
    Self: Sized
{
    type MacroedType = S;

    fn make_object<'a>(&self, ctx: &mut TaskContext<'a>) -> NeonResult<Handle<'a, JsObject>>;

    fn decode_fn_param(cx: &mut FunctionContext<'_>, param_idx: usize) -> eyre::Result<Self>;
}

pub trait AsNeonValue {
    type NeonValue: Value;

    fn as_neon_value<'a>(
        &self,
        ctx: &mut TaskContext<'a>
    ) -> NeonResult<Handle<'a, Self::NeonValue>>;
}

macro_rules! js_value {
    ($js_val:ident, [$($val:ident),*], $val_ident:ident, $ctx_ident:ident, $conversion:block) => {
        $(
            impl AsNeonValue for $val {
                type NeonValue = $js_val;

                fn as_neon_value<'a>(&self, ctx: &mut TaskContext<'a>) -> NeonResult<Handle<'a, Self::NeonValue>> {
                    let $val_ident = self;
                    let $ctx_ident = ctx;
                    Ok($conversion)
                }
            }
        )*
    };
}

js_value!(JsNumber, [u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64], this, ctx, {
    JsNumber::new(ctx, *this as f64)
});

js_value!(JsNumber, [U24], this, ctx, { JsNumber::new(ctx, this.to::<u64>() as f64) });

js_value!(JsNumber, [I24], this, ctx, {
    JsNumber::new(ctx, TryInto::<i32>::try_into(*this).unwrap() as f64)
});

js_value!(JsBigInt, [U256], this, ctx, {
    JsBigInt::from_digits_le(
        ctx,
        neon::types::bigint::Sign::Positive,
        &this.to_base_le(10).collect::<Vec<_>>()
    )
});

js_value!(JsBigInt, [I256], this, ctx, {
    let sign = if this.is_positive() {
        neon::types::bigint::Sign::Positive
    } else {
        neon::types::bigint::Sign::Negative
    };

    JsBigInt::from_digits_le(ctx, sign, this.as_limbs())
});

js_value!(JsUint8Array, [Bytes], this, ctx, { JsUint8Array::from_slice(ctx, &&*this)? });

js_value!(JsBoolean, [bool], this, ctx, { JsBoolean::new(ctx, *this) });

js_value!(JsString, [Address, B256], this, ctx, { JsString::new(ctx, format!("{:?}", this)) });

impl<A, B> AsNeonValue for HashMap<A, B>
where
    A: AsNeonValue,
    B: AsNeonValue
{
    type NeonValue = JsArray;

    fn as_neon_value<'a>(
        &self,
        ctx: &mut TaskContext<'a>
    ) -> NeonResult<Handle<'a, Self::NeonValue>> {
        let res = ctx.empty_array();

        for (i, (key, val)) in self.iter().enumerate() {
            let inner_obj = ctx.empty_object();
            let key_cast = key.as_neon_value(ctx)?;
            inner_obj.set(ctx, "key", key_cast)?;
            let val_cast = val.as_neon_value(ctx)?;
            inner_obj.set(ctx, "val", val_cast)?;
            res.set(ctx, i as u32, inner_obj)?;
        }
        Ok(res)
    }
}

impl<A: AsNeonValue> AsNeonValue for Option<A> {
    type NeonValue = JsValue;

    fn as_neon_value<'a>(
        &self,
        ctx: &mut TaskContext<'a>
    ) -> NeonResult<Handle<'a, Self::NeonValue>> {
        if let Some(val) = self.as_ref() {
            Ok(val.as_neon_value(ctx)?.as_value(ctx))
        } else {
            Ok(JsNull::new(ctx).as_value(ctx))
        }
    }
}

fn t() {
    let this = I256::default();
}
