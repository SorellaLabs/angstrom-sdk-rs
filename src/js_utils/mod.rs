use angstrom_types::contract_bindings::angstrom::Angstrom::PoolKey;
use neon::{
    object::Object,
    prelude::{Handle, TaskContext},
    result::NeonResult,
    types::{JsNumber, JsObject, JsString}
};

mod types;

pub trait MakeObject<S = Self>
where
    S: From<Self> + Clone,
    Self: Sized
{
    type MacroedType = S;

    fn to_object(&self, obj: &Handle<'_, JsObject>, ctx: &mut TaskContext<'_>) -> NeonResult<()>;
}
