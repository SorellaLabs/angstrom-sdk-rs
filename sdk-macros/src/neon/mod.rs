pub mod fn_param;
pub mod object;

use proc_macro2::{Span, TokenStream};
use syn::{
    parse::Parse, Data, DataEnum, DataStruct, DeriveInput, GenericArgument, Ident, Path,
    PathArguments, PathSegment, Token, Type, TypePath
};

pub fn parse(item: DeriveInput) -> syn::Result<TokenStream> {
    match &item.data {
        Data::Struct(data_struct) => parse_struct(&item, data_struct),
        Data::Enum(data_enum) => parse_enum(&item, data_enum),
        Data::Union(_) => unimplemented!()
    }
}

fn parse_struct(item: &DeriveInput, data_struct: &DataStruct) -> syn::Result<TokenStream> {
    let name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let fields_set = data_struct.fields.iter().filter_map(|field| {
        field.ident.as_ref().map(|field_name| {
            let name_str = field_name.to_string();
            quote::quote! {
                let val = crate::js_utils::AsNeonValue::as_neon_value(self, ctx)?;
                obj.set(ctx, #name_str, val)?;
            }
        })
    });

    let trait_impl = quote::quote! {
        impl #impl_generics crate::js_utils::MakeObject for #name #ty_generics #where_clause {
            fn make_object<'a>(&self, ctx: &mut neon::prelude::TaskContext<'a>) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, neon::prelude::JsObject>> {
                let obj = neon::context::Context::empty_object(ctx);
                #(#fields_set)*
                Ok(obj)
            }

            fn decode_fn_param(cx: &mut neon::prelude::FunctionContext<'_>, param_idx: usize) -> eyre::Result<Self> {
                let obj = cx.argument::<neon::prelude::JsObject>(param_idx)?;
                <Self as crate::js_utils::AsNeonValue>::from_neon_value(obj, cx)
            }
        }

        impl #impl_generics crate::js_utils::AsNeonValue for #name #ty_generics #where_clause {
            type NeonValue = neon::prelude::JsObject;

            fn as_neon_value<'a>(
                &self,
                ctx: &mut neon::prelude::TaskContext<'a>
            ) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, Self::NeonValue>> {
                crate::js_utils::MakeObject::make_object(self, ctx)
            }

            fn from_neon_value(
                value: neon::prelude::Handle<'_, Self::NeonValue>,
                cx: &mut neon::prelude::TaskContext<'_>
            ) -> NeonResult<Self>
            where
                Self: Sized {
                    todo!();
                }
        }

    };

    Ok(trait_impl)
}

fn parse_enum(item: &DeriveInput, data_enum: &DataEnum) -> syn::Result<TokenStream> {
    let name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let variant_tokens = data_enum
        .variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let variant_name_str = variant_name.to_string().to_lowercase();
            let fields = &variant.fields;
            let fields_set = fields
                .iter()
                .filter_map(|field| {
                    field.ident.as_ref().map(|field_name| {
                        let name_str = field_name.to_string();
                        quote::quote! {
                            let val = crate::js_utils::AsNeonValue::as_neon_value(self, ctx)?;
                            obj.set(ctx, #name_str, val)?;
                        }
                    })
                })
                .collect::<Vec<_>>();

            let field_names = fields
                .iter()
                .map(|field| field.ident.clone().unwrap())
                .collect::<Vec<_>>();

            quote::quote! {
                #name::#variant_name { #(#field_names),* } => {
                    let val = neon::prelude::JsString::new(ctx, #variant_name_str.to_string());
                    obj.set(ctx, "type", val)?;
                    #(#fields_set)*
                }
            }
        })
        .collect::<Vec<_>>();

    let trait_impl = quote::quote! {
        impl #impl_generics crate::js_utils::MakeObject for #name #ty_generics #where_clause {
            fn make_object<'a>(&self, ctx: &mut neon::prelude::TaskContext<'a>) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, neon::prelude::JsObject>> {
                let obj = neon::context::Context::empty_object(ctx);
                let me: Self = self.clone();
                match me {
                    #(#variant_tokens)*
                };

                Ok(obj)

            }

            fn decode_fn_param(cx: &mut neon::prelude::FunctionContext<'_>, param_idx: usize) -> eyre::Result<Self> {
                let obj = cx.argument::<neon::prelude::JsObject>(param_idx)?;
                <Self as crate::js_utils::AsNeonValue>::from_neon_value(obj, cx)
            }
        }

        impl #impl_generics crate::js_utils::AsNeonValue for #name #ty_generics #where_clause {
            type NeonValue = neon::prelude::JsObject;

            fn as_neon_value<'a>(
                &self,
                ctx: &mut neon::prelude::TaskContext<'a>
            ) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, Self::NeonValue>> {
                crate::js_utils::MakeObject::make_object(self, ctx)
            }

            fn from_neon_value(
                value: neon::prelude::Handle<'_, Self::NeonValue>,
                cx: &mut neon::prelude::TaskContext<'_>
            ) -> NeonResult<Self>
            where
                Self: Sized {
                    todo!();
                }
        }
    };

    Ok(trait_impl)
}
