pub mod fn_param;
pub mod object;

use proc_macro2::{Span, TokenStream};
use syn::{
    parse::Parse, Data, DataEnum, DataStruct, DeriveInput, GenericArgument, Ident, Path,
    PathArguments, PathSegment, Token, Type, TypePath
};

use crate::neon::object::{field_from_neon_value, field_to_neon_value};

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

    let (fields_to_set, fields_from_set): (Vec<_>, Vec<_>) = data_struct
        .fields
        .iter()
        .filter_map(|field| field_to_neon_value(field).zip(field_from_neon_value(field)))
        .unzip();

    let field_names = data_struct
        .fields
        .iter()
        .map(|field| field.ident.clone().unwrap())
        .collect::<Vec<_>>();

    let trait_impl = quote::quote! {
        impl #impl_generics crate::js_utils::MakeObject for #name #ty_generics #where_clause {
            fn make_object<'a, C: neon::prelude::Context<'a>>(&self, cx: &mut C) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, neon::prelude::JsObject>> {
                println!("MADE SELF");
                let obj = neon::context::Context::empty_object(cx);
                #(#fields_to_set)*
                println!("MADE OBJECT");
                Ok(obj)
            }

            fn decode_fn_param(cx: &mut neon::prelude::FunctionContext<'_>, param_idx: usize) -> neon::prelude::NeonResult<Self> {
                let obj = cx.argument::<neon::prelude::JsObject>(param_idx)?;
                <Self as crate::js_utils::AsNeonValue>::from_neon_value(obj, cx)
            }
        }

        impl #impl_generics crate::js_utils::AsNeonValue for #name #ty_generics #where_clause {
            type NeonValue = neon::prelude::JsObject;

            fn as_neon_value<'a, C: neon::prelude::Context<'a>>(
                &self,
                cx: &mut C
            ) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, Self::NeonValue>> {
                crate::js_utils::MakeObject::make_object(self, cx)
            }

            fn from_neon_value<'a, C: neon::prelude::Context<'a>>(
                value: neon::prelude::Handle<'a, Self::NeonValue>,
                cx: &mut C
            ) -> neon::result::NeonResult<Self>
            where
                Self: Sized {
                    #(#fields_from_set)*
                    Ok(Self {
                        #(#field_names),*
                    })
                }
        }

    };

    Ok(trait_impl)
}

fn parse_enum(item: &DeriveInput, data_enum: &DataEnum) -> syn::Result<TokenStream> {
    let name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let (variant_to_tokens, variant_from_tokens): (Vec<_>, Vec<_>) = data_enum
        .variants
        .iter()
        .map(|variant| {
            let variant_name = &variant.ident;
            let variant_name_str = variant_name.to_string().to_lowercase();
            let fields = &variant.fields;
            let (fields_to_set, fields_from_set): (Vec<_>, Vec<_>) = fields
                .iter()
                .filter_map(|field| field_to_neon_value(field).zip(field_from_neon_value(field)))
                .unzip();

            let field_names = fields
                .iter()
                .map(|field| field.ident.clone().unwrap())
                .collect::<Vec<_>>();

            (
                quote::quote! {
                    #name::#variant_name { #(#field_names),* } => {
                        let val = neon::prelude::JsString::new(cx, #variant_name_str.to_string());
                        obj.set(cx, "type", val)?;
                        #(#fields_to_set)*
                    }
                },
                quote::quote! {
                    variant_name_str => {
                        #(#fields_from_set)*
                        Ok(Self::#variant_name { #(#field_names),* })
                    }
                }
            )
        })
        .unzip();

    let trait_impl = quote::quote! {
        impl #impl_generics crate::js_utils::MakeObject for #name #ty_generics #where_clause {
            fn make_object<'a, C: neon::prelude::Context<'a>>(&self, cx: &mut C) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, neon::prelude::JsObject>> {
                let obj = neon::context::Context::empty_object(cx);
                let me: Self = self.clone();
                match me {
                    #(#variant_to_tokens)*
                };

                Ok(obj)

            }

            fn decode_fn_param(cx: &mut neon::prelude::FunctionContext<'_>, param_idx: usize) -> neon::prelude::NeonResult<Self> {
                let obj = cx.argument::<neon::prelude::JsObject>(param_idx)?;
                <Self as crate::js_utils::AsNeonValue>::from_neon_value(obj, cx)
            }
        }

        impl #impl_generics crate::js_utils::AsNeonValue for #name #ty_generics #where_clause {
            type NeonValue = neon::prelude::JsObject;

            fn as_neon_value<'a, C: neon::prelude::Context<'a>>(
                &self,
                cx: &mut C
            ) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, Self::NeonValue>> {
                crate::js_utils::MakeObject::make_object(self, cx)
            }

            fn from_neon_value<'a, C: neon::prelude::Context<'a>>(
                value: neon::prelude::Handle<'a, Self::NeonValue>,
                cx: &mut C
            ) -> neon::prelude::NeonResult<Self>
            where
                Self: Sized {
                    let variant_name = value
                        .get::<neon::types::JsString, _, _>(cx, "kind")?
                        .value(cx);


                    match variant_name.to_lowercase().as_str() {
                        #(#variant_from_tokens)*
                        _ => unreachable!("'{variant_name}' is not a valid variant")
                    }


                }
        }
    };

    Ok(trait_impl)
}
