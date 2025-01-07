use proc_macro2::{Span, TokenStream};
use syn::{
    parse::Parse, Data, DataEnum, DataStruct, DeriveInput, GenericArgument, Ident, PathArguments,
    PathSegment, Token, Type
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
            let rust_ty = RustTypes::from_macro_type(&field.ty);
            rust_ty.set_tokens_with_conversion(field_name, false)
        })
    });

    let trait_impl = quote::quote! {
        impl #impl_generics crate::js_utils::MakeObject for #name #ty_generics #where_clause {
            fn make_object(&self, obj: &neon::prelude::Handle<'_, neon::prelude::JsObject>, ctx: &mut neon::prelude::TaskContext<'_>) -> neon::prelude::NeonResult<()> {
                #(#fields_set)*

                Ok(())

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
                        let rust_ty = RustTypes::from_macro_type(&field.ty);
                        rust_ty.set_tokens_with_conversion(field_name, true)
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
                    obj.set(ctx, "variant_name", val)?;
                    #(#fields_set)*
                }
            }
        })
        .collect::<Vec<_>>();

    let trait_impl = quote::quote! {
        impl #impl_generics crate::js_utils::MakeObject for #name #ty_generics #where_clause {
            fn make_object(&self, obj: &neon::prelude::Handle<'_, neon::prelude::JsObject>, ctx: &mut neon::prelude::TaskContext<'_>) -> neon::prelude::NeonResult<()> {
                let me: Self = self.clone();
                match me {
                    #(#variant_tokens)*
                };

                Ok(())

            }
        }
    };

    Ok(trait_impl)
}

#[allow(non_camel_case_types)]
#[derive(Debug, Clone)]
enum RustTypes {
    /// native rust types
    u8,
    u16,
    u32,
    u64,
    u128,
    i8,
    i16,
    i32,
    i64,
    i128,
    f32,
    f64,
    bool,
    /// eth types
    Address,
    TxHash,
    B256,
    U24,
    U256,
    I24,
    I256,
    Option(Option<Box<RustTypes>>),
    HashMap(Option<(Box<RustTypes>, Box<RustTypes>)>),
    Bytes,
    Other
}

impl RustTypes {
    fn from_macro_type(ty: &Type) -> Self {
        match ty {
            // Type::Array(type_array) => todo!(),
            // Type::BareFn(type_bare_fn) => todo!(),
            // Type::Group(type_group) => todo!(),
            // Type::ImplTrait(type_impl_trait) => todo!(),
            // Type::Infer(type_infer) => todo!(),
            // Type::Macro(type_macro) => todo!(),
            // Type::Never(type_never) => todo!(),
            // Type::Paren(type_paren) => todo!(),
            Type::Path(type_path) => {
                if let Some(ty_lit) = type_path.path.get_ident() {
                    ty_lit.to_string().into()
                } else {
                    Self::parse_path_segments(type_path.path.segments.iter().rev())
                }
            }
            // Type::Ptr(type_ptr) => todo!(),
            // Type::Reference(type_reference) => todo!(),
            // Type::Slice(type_slice) => todo!(),
            // Type::TraitObject(type_trait_object) => todo!(),
            // Type::Tuple(type_tuple) => todo!(),
            // Type::Verbatim(token_stream) => todo!(),
            _ => unimplemented!()
        }
    }

    fn parse_path_segments<'a>(segments: impl Iterator<Item = &'a PathSegment>) -> Self {
        let me = segments
            .map(|seg| {
                let mut this_seg: Self = seg.ident.to_string().into();
                match &seg.arguments {
                    PathArguments::AngleBracketed(b) => {
                        let inner_segs = b
                            .args
                            .iter()
                            .filter_map(|arg| match arg {
                                GenericArgument::Type(ty) => Some(Self::from_macro_type(ty)),
                                _ => None
                            })
                            .collect::<Vec<_>>();
                        this_seg.add_from_inner(inner_segs);
                    }
                    PathArguments::Parenthesized(_) => unimplemented!(),
                    PathArguments::None => ()
                };

                this_seg
            })
            .collect::<Vec<_>>();

        me.first().unwrap().clone()
    }

    fn add_from_inner(&mut self, inner_segs: Vec<Self>) {
        match self {
            RustTypes::Option(rust_types) => {
                *rust_types = Some(Box::new(inner_segs.first().unwrap().clone()))
            }
            RustTypes::HashMap(rust_types) => {
                let mut iter_segs = inner_segs.into_iter();
                *rust_types = Some((
                    Box::new(iter_segs.next().unwrap().clone()),
                    Box::new(iter_segs.next().unwrap().clone())
                ))
            }
            _ => ()
        }
    }

    fn to_conversion_token(
        self,
        field_name: &Ident,
        is_enum: bool,
        with_append: Option<TokenStream>,
        with_loop_obj: Option<TokenStream>
    ) -> TokenStream {
        let name_str = field_name.to_string();
        let field_name_dt = if is_enum {
            quote::quote! {#field_name #with_append}
        } else {
            quote::quote! {self.#field_name #with_append}
        };

        let obj_name = with_loop_obj.unwrap_or(quote::quote! {obj});
        match self {
            RustTypes::u8
            | RustTypes::u16
            | RustTypes::u32
            | RustTypes::u64
            | RustTypes::u128
            | RustTypes::i8
            | RustTypes::i16
            | RustTypes::i32
            | RustTypes::i64
            | RustTypes::i128
            | RustTypes::f32
            | RustTypes::f64 => {
                quote::quote! {
                    let val = neon::prelude::JsNumber::new(ctx, #field_name_dt as f64);
                    #obj_name.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::Address | RustTypes::TxHash | RustTypes::B256 => quote::quote! {
                let val = neon::prelude::JsString::new(ctx, format!("{:?}", #field_name_dt));
                #obj_name.set(ctx, #name_str, val)?;
            },
            RustTypes::U24 => {
                quote::quote! {
                    let val = neon::prelude::JsNumber::new(ctx, #field_name_dt.to::<u64>() as f64);
                    #obj_name.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::I256 => {
                quote::quote! {
                    let this = #field_name_dt;
                    let sign = if this.is_positive() {
                        neon::types::bigint::Sign::Positive
                    } else {
                        neon::types::bigint::Sign::Negative
                    };

                    let val = neon::types::JsBigInt::from_digits_le(ctx, sign, &this.to_base_le(10).collect::<Vec<_>>());
                    #obj_name.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::U256 => {
                quote::quote! {
                    let val = neon::types::JsBigInt::from_digits_le(ctx, neon::types::bigint::Sign::Positive, &#field_name_dt.to_base_le(10).collect::<Vec<_>>());
                    #obj_name.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::I24 => {
                quote::quote! {
                    let val = neon::prelude::JsNumber::new(ctx, TryInto::<i64>::try_into(#field_name_dt).unwrap() as f64);
                    #obj_name.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::bool => {
                quote::quote! {
                   let val = neon::prelude::JsBoolean::new(ctx, #field_name_dt);
                    #obj_name.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::Bytes => {
                quote::quote! {
                   let val = neon::types::JsUint8Array::from_slice(ctx, &*#field_name_dt)?;
                    #obj_name.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::Option(option_val) => {
                let inner = option_val.unwrap().to_conversion_token(
                    field_name,
                    is_enum,
                    Some(quote::quote! {.as_ref().unwrap().clone()}),
                    None
                );
                quote::quote! {
                    //let this = #field_name_dt;
                    if #field_name_dt.is_none() {
                        let val = neon::types::JsNull::new(ctx);
                        #obj_name.set(ctx, #name_str, val)?;
                    } else {
                        #inner
                        #obj_name.set(ctx, #name_str, val)?;
                    }

                }
            }
            RustTypes::HashMap(option_val) => {
                let (inner0_ty, inner1_ty) = option_val.unwrap();
                let (key, val) =
                    (Ident::new("1", Span::call_site()), Ident::new("0", Span::call_site()));

                let inner0 = inner0_ty.to_conversion_token(
                    &key,
                    true,
                    None,
                    Some(quote::quote! {inner_obj})
                );
                let inner1 = inner1_ty.to_conversion_token(
                    &val,
                    true,
                    None,
                    Some(quote::quote! {inner_obj})
                );

                quote::quote! {
                    let res = a.empty_array();
                    for (i, (key, val)) in #field_name_dt.iter().enumerate() {
                        let inner_obj = a.empty_object();
                        #inner0;
                        #inner1;
                        res.set(ctx, i as u32, inner_obj)?;
                    }
                    #obj_name.set(ctx, #name_str, res)?;
                }
            }
            RustTypes::Other => {
                quote::quote! {
                    let this_obj = ctx.empty_object();
                    #field_name_dt.make_object(&this_obj, ctx)?;
                    #obj_name.set(ctx, #name_str, this_obj)?;
                }
            }
        }
    }

    fn set_tokens_with_conversion(self, field_name: &Ident, is_enum: bool) -> TokenStream {
        self.to_conversion_token(field_name, is_enum, None, None)
    }
}

impl From<String> for RustTypes {
    fn from(value: String) -> Self {
        match value.as_str() {
            "u8" => Self::u8,
            "u16" => Self::u16,
            "u32" => Self::u32,
            "u64" => Self::u64,
            "u128" => Self::u128,
            "i8" => Self::i8,
            "i16" => Self::i16,
            "i32" => Self::i32,
            "i64" => Self::i64,
            "i128" => Self::i128,
            "f32" => Self::f32,
            "f64" => Self::f64,
            "bool" => Self::bool,
            "Address" => Self::Address,
            "TxHash" => Self::TxHash,
            "B256" => Self::B256,
            "U24" => Self::U24,
            "U256" => Self::U256,
            "I24" => Self::I24,
            "I256" => Self::I256,
            "Option" => Self::Option(None),
            "Bytes" => Self::Bytes,
            "HashMap" => Self::HashMap(None),
            _ => Self::Other
        }
    }
}

pub struct NeonObjectAs {
    to_impl_ty:    Ident,
    conversion_ty: Ident
}

impl NeonObjectAs {
    pub fn expand(self) -> TokenStream {
        let a = self.to_impl_ty;
        let b = self.conversion_ty;
        quote::quote! {
            impl crate::js_utils::MakeObject<#b> for #a {
                fn make_object(&self, obj: &neon::prelude::Handle<'_, neon::prelude::JsObject>, ctx: &mut neon::prelude::TaskContext<'_>) -> neon::prelude::NeonResult<()> {
                    let me: Self = self.clone();
                    let this: #b = me.into();
                    this.make_object(obj, ctx)?;

                    Ok(())

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
