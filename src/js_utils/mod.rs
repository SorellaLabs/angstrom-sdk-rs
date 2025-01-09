use std::{
    collections::{HashMap, HashSet},
    hash::Hash
};

use alloy_eips::eip4844::BYTES_PER_BLOB;
use alloy_primitives::{
    aliases::{I24, U24},
    Address, Bytes, FixedBytes, B256, I256, U256
};
use neon::{
    object::Object,
    prelude::{Context, FunctionContext, Handle},
    result::{NeonResult, Throw},
    types::{
        buffer::TypedArray, JsArray, JsBigInt, JsBoolean, JsNull, JsNumber, JsObject, JsString,
        JsUint8Array, JsValue, Value
    }
};

mod types;
pub use types::{
    OrderBuilderAddLiquidityArgs, OrderBuilderExactFlashOrderArgs,
    OrderBuilderExactStandingOrderArgs, OrderBuilderPartialFlashOrderArgs,
    OrderBuilderPartialStandingOrderArgs, OrderBuilderRemoveLiquidityArgs,
    OrderBuilderTopOfBlockOrderArgs
};

pub trait MakeObject<S = Self>
where
    S: From<Self> + Into<Self> + AsNeonValue + Clone,
    Self: Sized
{
    type MacroedType = S;

    fn make_object<'a, C: Context<'a>>(&self, cx: &mut C) -> NeonResult<Handle<'a, JsObject>>;

    fn decode_fn_param(cx: &mut FunctionContext<'_>, param_idx: usize) -> NeonResult<Self>;
}

pub trait AsNeonValue {
    type NeonValue: Value;

    fn as_neon_value<'a, C: Context<'a>>(
        &self,
        cx: &mut C
    ) -> NeonResult<Handle<'a, Self::NeonValue>>;

    fn from_neon_value<'a, C: Context<'a>>(
        value: Handle<'a, Self::NeonValue>,
        cx: &mut C
    ) -> NeonResult<Self>
    where
        Self: Sized;
}

macro_rules! js_value {
    ($js_val:ident, [$($val:ident),*], $self_ident:ident, $cx_ident:ident, $from_val_ident:ident, $conversion_to:block, $conversion_from:block) => {
        $(
            impl AsNeonValue for $val {
                type NeonValue = $js_val;

                fn as_neon_value<'a, C: Context<'a>>(&self, cx: &mut C) -> NeonResult<Handle<'a, Self::NeonValue>> {
                    let $self_ident = self;
                    let $cx_ident = cx;
                    Ok($conversion_to)
                }

                fn from_neon_value<'a, C: Context<'a>>(value: Handle<'_, Self::NeonValue>, cx: &mut C) -> NeonResult<Self>
                where
                    Self: Sized {
                        let $from_val_ident = value;
                        let $cx_ident = cx;
                        let res = $conversion_from as $val;
                        Ok(res)
                }
            }
        )*
    };

    (FromStr | $js_val:ident, [$($val:ident),*], $self_ident:ident, $cx_ident:ident, $from_val_ident:ident, $conversion_to:block, $conversion_from:block) => {
        $(
            impl AsNeonValue for $val {
                type NeonValue = $js_val;

                fn as_neon_value<'a, C: Context<'a>>(&self, cx: &mut C) -> NeonResult<Handle<'a, Self::NeonValue>> {
                    let $self_ident = self;
                    let $cx_ident = cx;
                    Ok($conversion_to)
                }

                fn from_neon_value<'a, C: Context<'a>>(value: Handle<'_, Self::NeonValue>, cx: &mut C) -> NeonResult<Self>
                where
                    Self: Sized {
                        let $from_val_ident = value;
                        let $cx_ident = cx;
                        let res: $val = std::str::FromStr::from_str(&$conversion_from).expect(&format!("'{}' could not be cast from a string", $conversion_from));
                        Ok(res)
                }
            }
        )*
    };
}

js_value!(
    JsNumber,
    [u8, u16, u32, u64, u128, i8, i16, i32, i64, i128, f32, f64],
    this,
    cx,
    val,
    { JsNumber::new(cx, *this as f64) },
    { val.value(cx) }
);

js_value!(JsNumber, [U24], this, cx, val, { JsNumber::new(cx, this.to::<u64>() as f64) }, {
    U24::from(val.value(cx) as u64)
});

js_value!(
    JsNumber,
    [I24],
    this,
    cx,
    val,
    { JsNumber::new(cx, TryInto::<i32>::try_into(*this).unwrap() as f64) },
    { I24::try_from(val.value(cx) as i32).unwrap() }
);

js_value!(
    JsBigInt,
    [U256],
    this,
    cx,
    val,
    {
        JsBigInt::from_digits_le(
            cx,
            neon::types::bigint::Sign::Positive,
            &this.to_base_le(10).collect::<Vec<_>>()
        )
    },
    { U256::from_limbs_slice(&val.to_digits_le(cx).1) }
);

js_value!(
    JsBigInt,
    [I256],
    this,
    cx,
    val,
    {
        let sign = if this.is_positive() {
            neon::types::bigint::Sign::Positive
        } else {
            neon::types::bigint::Sign::Negative
        };

        JsBigInt::from_digits_le(cx, sign, this.as_limbs())
    },
    {
        let (sign, bytes) = val.to_digits_le(cx);
        if matches!(sign, neon::types::bigint::Sign::Positive) {
            I256::checked_from_sign_and_abs(
                alloy_primitives::Sign::Positive,
                U256::from_limbs_slice(&bytes)
            )
            .expect("I256 value could not be read")
        } else {
            I256::checked_from_sign_and_abs(
                alloy_primitives::Sign::Negative,
                U256::from_limbs_slice(&bytes)
            )
            .expect("I256 value could not be read")
        }
    }
);

js_value!(JsUint8Array, [Bytes], this, cx, val, { JsUint8Array::from_slice(cx, &&*this)? }, {
    Bytes::copy_from_slice(&val.as_slice(cx).into_iter().copied().collect::<Vec<_>>())
});

js_value!(JsBoolean, [bool], this, cx, val, { JsBoolean::new(cx, *this) }, { val.value(cx) });

js_value!(
    FromStr | JsString,
    [Address, B256],
    this,
    cx,
    val,
    { JsString::new(cx, format!("{:?}", this)) },
    { val.value(cx) }
);

js_value!(JsString, [String], this, cx, val, { JsString::new(cx, this) }, { val.value(cx) });

impl<A, B> AsNeonValue for HashMap<A, B>
where
    A: AsNeonValue + Eq + Hash,
    B: AsNeonValue
{
    type NeonValue = JsArray;

    fn as_neon_value<'a, C: Context<'a>>(
        &self,
        cx: &mut C
    ) -> NeonResult<Handle<'a, Self::NeonValue>> {
        let res = cx.empty_array();

        for (i, (key, val)) in self.iter().enumerate() {
            let inner_obj = cx.empty_object();
            let key_cast = key.as_neon_value(cx)?;
            inner_obj.set(cx, "key", key_cast)?;
            let val_cast = val.as_neon_value(cx)?;
            inner_obj.set(cx, "val", val_cast)?;
            res.set(cx, i as u32, inner_obj)?;
        }
        Ok(res)
    }

    fn from_neon_value<'a, C: Context<'a>>(
        value: Handle<'a, Self::NeonValue>,
        cx: &mut C
    ) -> NeonResult<Self>
    where
        Self: Sized
    {
        Ok(value
            .to_vec(cx)?
            .into_iter()
            .map(|val| {
                val.downcast_or_throw::<JsObject, _>(cx).map(|obj| {
                    let key_obj = obj.get::<<A as AsNeonValue>::NeonValue, _, _>(cx, "key")?;
                    let key = A::from_neon_value(key_obj, cx)?;

                    let val_obj = obj.get::<<B as AsNeonValue>::NeonValue, _, _>(cx, "val")?;
                    let val = B::from_neon_value(val_obj, cx)?;

                    Ok::<_, Throw>((key, val))
                })
            })
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .collect())
    }
}

impl<A: AsNeonValue> AsNeonValue for Option<A> {
    type NeonValue = JsValue;

    fn as_neon_value<'a, C: Context<'a>>(
        &self,
        cx: &mut C
    ) -> NeonResult<Handle<'a, Self::NeonValue>> {
        if let Some(val) = self.as_ref() {
            Ok(val.as_neon_value(cx)?.as_value(cx))
        } else {
            Ok(JsNull::new(cx).as_value(cx))
        }
    }

    fn from_neon_value<'a, C: Context<'a>>(
        value: Handle<'a, Self::NeonValue>,
        cx: &mut C
    ) -> NeonResult<Self>
    where
        Self: Sized
    {
        if let Ok(val) = value.downcast::<<A as AsNeonValue>::NeonValue, _>(cx) {
            A::from_neon_value(val, cx).map(Some)
        } else {
            Ok(None)
        }
    }
}

impl<A: AsNeonValue> AsNeonValue for Vec<A> {
    type NeonValue = JsArray;

    fn as_neon_value<'a, C: Context<'a>>(
        &self,
        cx: &mut C
    ) -> NeonResult<Handle<'a, Self::NeonValue>> {
        let arr = cx.empty_array();
        for (i, val) in self.iter().enumerate() {
            let obj = val.as_neon_value(cx)?;
            arr.set(cx, i as u32, obj)?;
        }
        Ok(arr)
    }

    fn from_neon_value<'a, C: Context<'a>>(
        value: Handle<'a, Self::NeonValue>,
        cx: &mut C
    ) -> NeonResult<Self>
    where
        Self: Sized
    {
        value
            .to_vec(cx)?
            .into_iter()
            .map(|val| Ok(A::from_neon_value(val.downcast_or_throw(cx)?, cx)).flatten())
            .collect::<Result<Vec<_>, _>>()
    }
}

impl<A: AsNeonValue + Eq + Hash + Clone> AsNeonValue for HashSet<A> {
    type NeonValue = JsArray;

    fn as_neon_value<'a, C: Context<'a>>(
        &self,
        cx: &mut C
    ) -> NeonResult<Handle<'a, Self::NeonValue>> {
        self.into_iter()
            .map(Clone::clone)
            .collect::<Vec<_>>()
            .as_neon_value(cx)
    }

    fn from_neon_value<'a, C: Context<'a>>(
        value: Handle<'a, Self::NeonValue>,
        cx: &mut C
    ) -> NeonResult<Self>
    where
        Self: Sized
    {
        let vec_vals: Vec<A> = Vec::<A>::from_neon_value(value, cx)?;
        Ok(HashSet::from_iter(vec_vals))
    }
}

///
///
///
///
///
///
/// MACRO THIS FOR TUPLES!!!
impl<A: AsNeonValue, B: AsNeonValue> AsNeonValue for (A, B) {
    type NeonValue = JsObject;

    fn as_neon_value<'a, C: Context<'a>>(
        &self,
        cx: &mut C
    ) -> NeonResult<Handle<'a, Self::NeonValue>> {
        let obj = cx.empty_object();

        let val0 = self.0.as_neon_value(cx)?;
        obj.set(cx, 0, val0)?;

        let val1 = self.1.as_neon_value(cx)?;
        obj.set(cx, 1, val1)?;

        Ok(obj)
    }

    fn from_neon_value<'a, C: Context<'a>>(
        value: Handle<'a, Self::NeonValue>,
        cx: &mut C
    ) -> NeonResult<Self>
    where
        Self: Sized
    {
        let val0 = value.get::<<A as AsNeonValue>::NeonValue, _, _>(cx, 0)?;
        let val1 = value.get::<<B as AsNeonValue>::NeonValue, _, _>(cx, 1)?;

        Ok((A::from_neon_value(val0, cx)?, B::from_neon_value(val1, cx)?))
    }
}

impl AsNeonValue for () {
    type NeonValue = JsObject;

    fn as_neon_value<'a, C: Context<'a>>(
        &self,
        cx: &mut C
    ) -> NeonResult<Handle<'a, Self::NeonValue>> {
        Ok(cx.empty_object())
    }

    fn from_neon_value<'a, C: Context<'a>>(
        _: Handle<'a, Self::NeonValue>,
        _: &mut C
    ) -> NeonResult<Self>
    where
        Self: Sized
    {
        Ok(())
    }
}

/*

include in declarative macro later - was lazy
*/

impl AsNeonValue for FixedBytes<48> {
    type NeonValue = JsString;

    fn as_neon_value<'a, C: Context<'a>>(
        &self,
        cx: &mut C
    ) -> NeonResult<Handle<'a, Self::NeonValue>> {
        Ok(JsString::new(cx, format!("{:?}", self)))
    }

    fn from_neon_value<'a, C: Context<'a>>(
        value: Handle<'a, Self::NeonValue>,
        cx: &mut C
    ) -> NeonResult<Self>
    where
        Self: Sized
    {
        let str_val = value.value(cx);
        Ok(str_val.parse().expect("could not parse FixedBytes<48>"))
    }
}

impl AsNeonValue for FixedBytes<BYTES_PER_BLOB> {
    type NeonValue = JsString;

    fn as_neon_value<'a, C: Context<'a>>(
        &self,
        cx: &mut C
    ) -> NeonResult<Handle<'a, Self::NeonValue>> {
        Ok(JsString::new(cx, format!("{:?}", self)))
    }

    fn from_neon_value<'a, C: Context<'a>>(
        value: Handle<'a, Self::NeonValue>,
        cx: &mut C
    ) -> NeonResult<Self>
    where
        Self: Sized
    {
        let str_val = value.value(cx);
        Ok(str_val
            .parse()
            .expect("could not parse FixedBytes<BYTES_PER_BLOB>"))
    }
}
