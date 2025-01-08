use neon::{
    prelude::{Handle, TaskContext},
    result::NeonResult,
    types::JsObject
};

mod types;

pub trait MakeObject<S = Self>
where
    S: From<Self> + Clone,
    Self: Sized
{
    type MacroedType = S;

    fn make_object<'a>(self, ctx: &mut TaskContext<'a>) -> NeonResult<Handle<'a, JsObject>>;
}
