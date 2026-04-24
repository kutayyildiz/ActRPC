use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, parse_macro_input};

use crate::shared::{extract_named_struct_fields, field_name, is_option, value_descriptor_tokens};

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
    let fields = extract_named_struct_fields(input)?;

    let mut required = Vec::new();
    let mut optional = Vec::new();

    for field in &fields.named {
        let name = field_name(field)?;

        if let Some(inner) = is_option(&field.ty) {
            let ty = value_descriptor_tokens(inner, false)?;
            optional.push(quote! {
                ::actrpc_core::action::action_descriptor::types::FieldDescriptor {
                    name: #name.to_string(),
                    ty: #ty,
                }
            });
        } else {
            let ty = value_descriptor_tokens(&field.ty, false)?;
            required.push(quote! {
                ::actrpc_core::action::action_descriptor::types::FieldDescriptor {
                    name: #name.to_string(),
                    ty: #ty,
                }
            });
        }
    }

    Ok(quote! {
        impl ::actrpc_core::action::action_descriptor::traits::DescribeParams for #ident {
            fn describe_params() -> Option<
                ::actrpc_core::action::action_descriptor::types::ParamsDescriptor
            > {
                Some(
                    ::actrpc_core::action::action_descriptor::types::ParamsDescriptor::Object(
                        ::actrpc_core::action::action_descriptor::types::ParamsObjectDescriptor {
                            required_fields: vec![#(#required),*],
                            optional_fields: vec![#(#optional),*],
                        }
                    )
                )
            }
        }
    })
}
