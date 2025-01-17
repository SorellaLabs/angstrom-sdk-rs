use proc_macro2::TokenStream;
use syn::{parse::Parse, Data, DataEnum, DataStruct, DeriveInput, Field, Token, Type};

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
        .filter_map(|field| field_to_neon_value(field, false).zip(field_from_neon_value(field)))
        .unzip();

    let field_names = data_struct
        .fields
        .iter()
        .map(|field| field.ident.clone().unwrap())
        .collect::<Vec<_>>();

    let trait_impl = quote::quote! {
        impl #impl_generics crate::js_utils::MakeNeonObject for #name #ty_generics #where_clause {
            fn make_object<'a, C: neon::prelude::Context<'a>>(&self, cx: &mut C) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, neon::prelude::JsObject>> {
                let obj = neon::context::Context::empty_object(cx);
                #(#fields_to_set)*
                Ok(obj)
            }
        }

        impl #impl_generics crate::js_utils::AsNeonValue for #name #ty_generics #where_clause {
            type NeonValue = neon::prelude::JsObject;

            fn as_neon_value<'a, C: neon::prelude::Context<'a>>(
                &self,
                cx: &mut C
            ) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, Self::NeonValue>> {
                crate::js_utils::MakeNeonObject::make_object(self, cx)
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
                .filter_map(|field| {
                    field_to_neon_value(field, true).zip(field_from_neon_value(field))
                })
                .unzip();

            let field_names = fields
                .iter()
                .map(|field| {
                    field
                        .ident
                        .clone()
                        .expect("enum cannot have unnamed parameters in their variants")
                })
                .collect::<Vec<_>>();

            if field_names.len() > 1 {
                panic!(
                    "for parsing simplicity, enums can have 0 or 1 named params in their \
                     variants. If there are multiple, create a struct containing the params and \
                     set a `param` parameter as the only named param in the enum variant"
                );
            }

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
                        println!("IS VARIANT: {}", variant_name_str);
                        #(#fields_from_set)*
                        Ok(Self::#variant_name { #(#field_names),* })
                    }
                }
            )
        })
        .unzip();

    let trait_impl = quote::quote! {
        impl #impl_generics crate::js_utils::MakeNeonObject for #name #ty_generics #where_clause {
            fn make_object<'a, C: neon::prelude::Context<'a>>(&self, cx: &mut C) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, neon::prelude::JsObject>> {
                let obj = neon::context::Context::empty_object(cx);
                let me: Self = self.clone();
                match me {
                    #(#variant_to_tokens)*
                };

                Ok(obj)

            }
        }

        impl #impl_generics crate::js_utils::AsNeonValue for #name #ty_generics #where_clause {
            type NeonValue = neon::prelude::JsObject;

            fn as_neon_value<'a, C: neon::prelude::Context<'a>>(
                &self,
                cx: &mut C
            ) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, Self::NeonValue>> {
                crate::js_utils::MakeNeonObject::make_object(self, cx)
            }

            fn from_neon_value<'a, C: neon::prelude::Context<'a>>(
                value: neon::prelude::Handle<'a, Self::NeonValue>,
                cx: &mut C
            ) -> neon::prelude::NeonResult<Self>
            where
                Self: Sized {
                    let variant_name = value
                        .get::<neon::types::JsString, _, _>(cx, "type")?
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

pub(super) fn field_to_neon_value(field: &Field, is_enum: bool) -> Option<TokenStream> {
    field.ident.as_ref().map(|field_name| {
        let name_str = field_name.to_string();
        let field_ident = if is_enum {
            quote::quote! {#field_name}
        } else {
            quote::quote! {self.#field_name}
        };
        quote::quote! {
            let val = crate::js_utils::AsNeonValue::as_neon_value(&#field_ident, cx)?;
            obj.set(cx, #name_str, val)?;
        }
    })
}

pub(super) fn field_from_neon_value(field: &Field) -> Option<TokenStream> {
    field.ident.as_ref().map(|field_name| {
        let field_name_str = field_name.to_string();
        let field_ty = &field.ty;
        println!("{:?}", field_ty);
        quote::quote! {
            let field_name_obj = value.get::<<#field_ty as crate::js_utils::AsNeonValue>::NeonValue, _, _>(cx, #field_name_str).expect(&format!("could not get field name {}", #field_name_str));
            println!("converting {}: {}", #field_name_str, #field_ty_str);
            let #field_name = crate::js_utils::AsNeonValue::from_neon_value(field_name_obj, cx).expect(&format!("could not convert field name {}", #field_name_str));
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
            impl crate::js_utils::MakeNeonObject<#b> for #a {
                fn make_object<'a, C: neon::prelude::Context<'a>>(&self, cx: &mut C) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, neon::prelude::JsObject>> {
                    let me: Self = self.clone();
                    let this: #b = me.into();
                    Ok(this.make_object(cx)?)

                }
            }

            impl crate::js_utils::AsNeonValue for #a {
                type NeonValue = <#b as crate::js_utils::AsNeonValue>::NeonValue;

                fn as_neon_value<'a, C: neon::prelude::Context<'a>>(
                    &self,
                    cx: &mut C
                ) -> neon::prelude::NeonResult<neon::prelude::Handle<'a, Self::NeonValue>> {
                    crate::js_utils::MakeNeonObject::make_object(self, cx)
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

                fn decode_fn_param(cx: &mut neon::prelude::FunctionContext<'_>, param_idx: usize) -> neon::prelude::NeonResult<Self> {
                    let obj = cx.argument::<neon::prelude::JsObject>(param_idx)?;
                    let this = <#b as crate::js_utils::AsNeonValue>::from_neon_value(obj, cx)?;
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
