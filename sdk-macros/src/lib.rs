mod neon_obj;

use neon_obj::NeonObjectAs;
use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(NeonObject)]
pub fn neon_object(raw: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(raw as DeriveInput);
    neon_obj::parse(input)
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro]
///
/// used for types that aren't defined in the crate and can't be used with the
/// #[derive(NeonObject)] macro
///
/// create a type A that implements From<B>, where A is the outside-of-crate
/// defined type, and type B is inside the crate. Type B should have the
/// #[derive(NeonObject)] macro
///
/// i.e. does:
/// impl MakeObject<B> for A {...}
///
/// and calls Into<B>, then sets the values of type B in the JsObject
pub fn neon_object_as(input: TokenStream) -> TokenStream {
    parse_macro_input!(input as NeonObjectAs).expand().into()
}
