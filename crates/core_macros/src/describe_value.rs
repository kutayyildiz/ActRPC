// crates/core_macros/src/describe_value.rs
use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Fields, parse_macro_input};

use crate::shared::{
    extract_enum_variants, extract_named_struct_fields, field_name, value_descriptor_tokens,
};

pub fn expand(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let expanded = match expand_impl(&input) {
        Ok(tokens) => tokens,
        Err(err) => err.to_compile_error(),
    };

    TokenStream::from(expanded)
}

fn expand_impl(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let ident = &input.ident;

    let body = match &input.data {
        Data::Struct(_) => describe_struct_value(input)?,
        Data::Enum(_) => describe_enum_value(input)?,
        Data::Union(_) => {
            return Err(Error::new_spanned(input, "unions are not supported"));
        }
    };

    Ok(quote! {
        impl ::actrpc_core::action::action_descriptor::traits::DescribeValue for #ident {
            fn describe_value() -> ::actrpc_core::action::action_descriptor::types::ValueDescriptor {
                #body
            }
        }
    })
}

fn describe_struct_value(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let fields = extract_named_struct_fields(input)?;

    let field_tokens = fields
        .named
        .iter()
        .map(|field| {
            let name = field_name(field)?;
            let ty = value_descriptor_tokens(&field.ty, false)?;

            Ok(quote! {
                ::actrpc_core::action::action_descriptor::types::FieldDescriptor {
                    name: #name.to_string(),
                    ty: #ty,
                }
            })
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        ::actrpc_core::action::action_descriptor::types::ValueDescriptor::Object(
            ::actrpc_core::action::action_descriptor::types::NestedObjectDescriptor {
                fields: vec![#(#field_tokens),*],
            }
        )
    })
}

fn describe_enum_value(input: &DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let variants = extract_enum_variants(input)?;

    if variants.is_empty() {
        return Err(Error::new_spanned(
            input,
            "DescribeValue does not support empty enums",
        ));
    }

    let variant_tokens = variants
        .iter()
        .map(|variant| match &variant.fields {
            Fields::Unnamed(fields) if fields.unnamed.len() == 1 => {
                let ty = &fields.unnamed.first().expect("len checked").ty;
                value_descriptor_tokens(ty, false)
            }
            Fields::Named(_) => Err(Error::new_spanned(
                variant,
                "enum variants with named fields are not supported by DescribeValue; use a separate struct type per variant",
            )),
            Fields::Unnamed(_) => Err(Error::new_spanned(
                variant,
                "enum variants must have exactly one unnamed field",
            )),
            Fields::Unit => Err(Error::new_spanned(
                variant,
                "unit variants are not supported by DescribeValue",
            )),
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok(quote! {
        ::actrpc_core::action::action_descriptor::types::ValueDescriptor::OneOf(
            vec![#(#variant_tokens),*]
        )
    })
}
