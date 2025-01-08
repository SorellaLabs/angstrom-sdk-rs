use proc_macro2::{Span, TokenStream};
use syn::{
    parse::Parse, Data, DataEnum, DataStruct, DeriveInput, GenericArgument, Ident, Path,
    PathArguments, PathSegment, Token, Type, TypePath
};

pub struct NeonObjectAs {
    to_impl_ty:    Type,
    conversion_ty: Type
}

impl NeonObjectAs {
    pub fn expand(self) -> TokenStream {
        let a = self.to_impl_ty;
        let b = self.conversion_ty;
        quote::quote! {
            impl crate::js_utils::MakeObject<#b> for #a {
                fn make_object<'a>(&self, ctx: &mut neon::prelude::TaskContext<'a>) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, neon::prelude::JsObject>> {
                    let me: Self = self.clone();
                    let this: #b = me.into();
                    Ok(this.make_object(ctx)?)

                }

                fn decode_fn_param(cx: &mut neon::prelude::FunctionContext<'_>, param_idx: usize) -> eyre::Result<Self> {
                    todo!()
                }
            }
        }
    }
}

impl Parse for NeonObjectAs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let to_impl_ty = input.parse()?;

        input.parse::<Token![,]>()?;
        let conversion_ty = input.parse()?;

        Ok(Self { to_impl_ty, conversion_ty })
    }
}
