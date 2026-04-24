use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, parse_macro_input};

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

    let field_tokens = fields
        .named
        .iter()
        .map(|field| {
            if is_option(&field.ty).is_some() {
                return Err(Error::new_spanned(
                    &field.ty,
                    "DescribeOk does not allow Option<T> fields",
                ));
            }

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
        impl ::actrpc_core::action::action_descriptor::traits::DescribeOk for #ident {
            fn describe_ok() -> Option<
                ::actrpc_core::action::action_descriptor::types::OkDescriptor
            > {
                Some(
                    ::actrpc_core::action::action_descriptor::types::ValueDescriptor::Object(
                        ::actrpc_core::action::action_descriptor::types::NestedObjectDescriptor {
                            fields: vec![#(#field_tokens),*],
                        }
                    )
                )
            }
        }
    })
}
