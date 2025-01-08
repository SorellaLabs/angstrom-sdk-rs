use proc_macro2::{Span, TokenStream};
use syn::{
    parse::Parse, Data, DataEnum, DataStruct, DeriveInput, Field, GenericArgument, Ident, Path,
    PathArguments, PathSegment, Token, Type, TypePath
};

pub(super) fn field_to_neon_value(field: &Field) -> Option<TokenStream> {
    field.ident.as_ref().map(|field_name| {
        let name_str = field_name.to_string();
        quote::quote! {
            let val = crate::js_utils::AsNeonValue::as_neon_value(self, cx)?;
            obj.set(cx, #name_str, val)?;
        }
    })
}

pub(super) fn field_from_neon_value(field: &Field) -> Option<TokenStream> {
    field.ident.as_ref().map(|field_name| {
        let field_name_str = field_name.to_string();
        let field_ty = &field.ty;
        quote::quote! {
            let field_name_obj = value.get::<<#field_ty as crate::js_utils::AsNeonValue>::NeonValue, _, _>(cx, #field_name_str)?;
            let #field_name = crate::js_utils::AsNeonValue::from_neon_value(field_name_obj, cx)?;
        }
    })
}

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
                fn make_object<'a>(&self, cx: &mut neon::prelude::TaskContext<'a>) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, neon::prelude::JsObject>> {
                    let me: Self = self.clone();
                    let this: #b = me.into();
                    Ok(this.make_object(cx)?)

                }

                fn decode_fn_param(cx: &mut neon::prelude::FunctionContext<'_>, param_idx: usize) -> neon::prelude::NeonResult<Self> {
                    let obj = cx.argument::<neon::prelude::JsObject>(param_idx)?;
                    let this = <#b as crate::js_utils::AsNeonValue>::from_neon_value(obj, cx)?;
                    Ok(this.into())
                }
            }

            impl crate::js_utils::AsNeonValue for #a {
                type NeonValue = <#b as crate::js_utils::AsNeonValue>::NeonValue;

                fn as_neon_value<'a>(
                    &self,
                    cx: &mut neon::prelude::TaskContext<'a>
                ) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, Self::NeonValue>> {
                    crate::js_utils::MakeObject::make_object(self, cx)
                }

                fn from_neon_value<'a, C: neon::prelude::Context<'a>>(
                    value: neon::prelude::Handle<'a, Self::NeonValue>,
                    cx: &mut C
                ) -> neon::prelude::NeonResult<Self>
                where
                    Self: Sized {
                        let this = <#b as crate::js_utils::AsNeonValue>::from_neon_value(value, cx)?;
                        Ok(this.into())
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
