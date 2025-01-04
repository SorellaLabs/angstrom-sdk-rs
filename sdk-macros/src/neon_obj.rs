use proc_macro2::TokenStream;
use syn::{
    parse::Parse, Data, DataStruct, DeriveInput, GenericArgument, Ident, PathArguments,
    PathSegment, Token, Type
};

pub fn parse(item: DeriveInput) -> syn::Result<TokenStream> {
    match &item.data {
        Data::Struct(data_struct) => parse_struct(&item, data_struct),
        Data::Enum(_) => unimplemented!(),
        Data::Union(_) => unimplemented!()
    }
}

fn parse_struct(item: &DeriveInput, data_struct: &DataStruct) -> syn::Result<TokenStream> {
    let name = &item.ident;
    let (impl_generics, ty_generics, where_clause) = item.generics.split_for_impl();

    let fields_set = data_struct.fields.iter().filter_map(|field| {
        field.ident.as_ref().map(|field_name| {
            let rust_ty = RustTypes::from_macro_type(&field.ty);
            rust_ty.set_tokens_with_conversion(field_name)
        })
    });

    let trait_impl = quote::quote! {
        impl #impl_generics crate::js_utils::MakeObject for #name #ty_generics #where_clause {
            fn to_object(&self, obj: &neon::prelude::Handle<'_, neon::prelude::JsObject>, ctx: &mut neon::prelude::TaskContext<'_>) -> neon::prelude::NeonResult<()> {
                #(#fields_set)*

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
    Option(Option<Box<RustTypes>>),
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
            _ => ()
        }
    }

    fn to_conversion_token(
        self,
        field_name: &Ident,
        with_append: Option<TokenStream>
    ) -> TokenStream {
        let name_str = field_name.to_string();
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
                    let val = neon::prelude::JsNumber::new(ctx, self.#field_name as f64);
                    obj.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::Address | RustTypes::TxHash | RustTypes::B256 => quote::quote! {
                let val = neon::prelude::JsString::new(ctx, format!("{:?}", self.#field_name #with_append));
                    obj.set(ctx, #name_str, val)?;
            },
            RustTypes::U24 => {
                quote::quote! {
                    let val = neon::prelude::JsNumber::new(ctx, self.#field_name.to::<u64>() as f64);
                    obj.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::U256 => {
                quote::quote! {
                    let this = self.#field_name #with_append;
                    let sign = if this.is_positive() {
                        neon::types::bigint::Sign::Positive
                    } else {
                        neon::types::bigint::Sign::Negative
                    };

                    let val = neon::types::JsBigInt::from_digits_le(ctx, sign, this.to_base_le(10));
                    obj.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::I24 => {
                quote::quote! {
                    let val = neon::prelude::JsNumber::new(ctx, TryInto::<i64>::try_into(self.#field_name #with_append).unwrap() as f64);
                    obj.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::bool => {
                quote::quote! {
                   let val = neon::prelude::JsBoolean::new(ctx, self.#field_name #with_append);
                    obj.set(ctx, #name_str, val)?;
                }
            }
            RustTypes::Option(option_val) => {
                let inner = option_val
                    .unwrap()
                    .to_conversion_token(field_name, Some(quote::quote! {.as_ref().unwrap()}));
                quote::quote! {
                    let this = self.#field_name #with_append;
                    if this.is_none() {
                        let val = neon::types::JsNull::new(ctx);
                        obj.set(ctx, #name_str, val)?;
                        //val
                    } else {
                        #inner
                        obj.set(ctx, #name_str, val)?;
                    }

                }
            }
            RustTypes::Other => {
                quote::quote! {
                    self.#field_name.to_object(obj, ctx)
                }
            }
        }
    }

    fn set_tokens_with_conversion(self, field_name: &Ident) -> TokenStream {
        self.to_conversion_token(field_name, None)
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
            "Option" => Self::Option(None),
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
                fn to_object(&self, obj: &neon::prelude::Handle<'_, neon::prelude::JsObject>, ctx: &mut neon::prelude::TaskContext<'_>) -> neon::prelude::NeonResult<()> {
                    let this: #b = self.clone().into();
                    this.to_object(obj, ctx)?;

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
