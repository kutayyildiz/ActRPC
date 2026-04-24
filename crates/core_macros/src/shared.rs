use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Data, DataEnum, DataStruct, DeriveInput, Error, Field, Fields, FieldsNamed, GenericArgument,
    PathArguments, Result, Type, TypePath, Variant,
};

pub fn extract_named_struct_fields(input: &DeriveInput) -> Result<&FieldsNamed> {
    match &input.data {
        Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) => Ok(fields),
        Data::Struct(_) => Err(Error::new_spanned(
            input,
            "only structs with named fields are supported",
        )),
        Data::Enum(_) => Err(Error::new_spanned(
            input,
            "enums are not supported by this derive",
        )),
        Data::Union(_) => Err(Error::new_spanned(input, "unions are not supported")),
    }
}

pub fn extract_enum_variants(
    input: &DeriveInput,
) -> Result<&syn::punctuated::Punctuated<Variant, syn::token::Comma>> {
    match &input.data {
        Data::Enum(DataEnum { variants, .. }) => Ok(variants),
        Data::Struct(_) => Err(Error::new_spanned(
            input,
            "only enums are supported by this derive path",
        )),
        Data::Union(_) => Err(Error::new_spanned(input, "unions are not supported")),
    }
}

pub fn field_name(field: &Field) -> Result<String> {
    let ident = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "only named fields are supported"))?;
    Ok(ident.to_string())
}

pub fn is_option(ty: &Type) -> Option<&Type> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };

    let segment = path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }

    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };

    if args.args.len() != 1 {
        return None;
    }

    match args.args.first()? {
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}

pub fn is_vec(ty: &Type) -> Option<&Type> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };

    let segment = path.segments.last()?;
    if segment.ident != "Vec" {
        return None;
    }

    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };

    if args.args.len() != 1 {
        return None;
    }

    match args.args.first()? {
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}

pub fn is_map(ty: &Type) -> Option<&Type> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };

    let segment = path.segments.last()?;
    let ident = segment.ident.to_string();
    if ident != "HashMap" && ident != "BTreeMap" {
        return None;
    }

    let PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };

    if args.args.len() != 2 {
        return None;
    }

    let mut iter = args.args.iter();
    let key = match iter.next()? {
        GenericArgument::Type(inner) => inner,
        _ => return None,
    };
    let value = match iter.next()? {
        GenericArgument::Type(inner) => inner,
        _ => return None,
    };

    if !is_string_type(key) {
        return None;
    }

    Some(value)
}

fn is_string_type(ty: &Type) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };

    path.segments
        .last()
        .map(|segment| segment.ident == "String")
        .unwrap_or(false)
}

fn primitive_descriptor_tokens(ty: &Type) -> Option<TokenStream> {
    let Type::Path(TypePath { path, .. }) = ty else {
        return None;
    };

    let ident = &path.segments.last()?.ident;
    let primitive = match ident.to_string().as_str() {
        "bool" => quote! {
            ::actrpc_core::action::action_descriptor::types::PrimitiveDescriptor::Bool
        },
        "String" => quote! {
            ::actrpc_core::action::action_descriptor::types::PrimitiveDescriptor::String
        },
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" | "i8" | "i16" | "i32" | "i64" | "i128"
        | "isize" => quote! {
            ::actrpc_core::action::action_descriptor::types::PrimitiveDescriptor::Integer
        },
        "f32" | "f64" => quote! {
            ::actrpc_core::action::action_descriptor::types::PrimitiveDescriptor::Number
        },
        _ => return None,
    };

    Some(quote! {
        ::actrpc_core::action::action_descriptor::types::ValueDescriptor::Primitive(#primitive)
    })
}

fn is_serde_json_value(ty: &Type) -> bool {
    let Type::Path(TypePath { path, .. }) = ty else {
        return false;
    };

    let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();

    matches!(
        segments.as_slice(),
        [single] if single == "Value"
    ) || matches!(
        segments.as_slice(),
        [a, b] if a == "serde_json" && b == "Value"
    )
}

pub fn value_descriptor_tokens(ty: &Type, allow_option: bool) -> Result<TokenStream> {
    if let Some(inner) = is_option(ty) {
        if allow_option {
            return value_descriptor_tokens(inner, false);
        }

        return Err(Error::new_spanned(ty, "Option<T> is not allowed here"));
    }

    if is_serde_json_value(ty) {
        return Ok(quote! {
            ::actrpc_core::action::action_descriptor::types::ValueDescriptor::Any
        });
    }

    if let Some(primitive) = primitive_descriptor_tokens(ty) {
        return Ok(primitive);
    }

    if let Some(inner) = is_vec(ty) {
        let inner_tokens = value_descriptor_tokens(inner, false)?;
        return Ok(quote! {
            ::actrpc_core::action::action_descriptor::types::ValueDescriptor::Array(
                Box::new(#inner_tokens)
            )
        });
    }

    if let Some(inner) = is_map(ty) {
        let inner_tokens = value_descriptor_tokens(inner, false)?;
        return Ok(quote! {
            ::actrpc_core::action::action_descriptor::types::ValueDescriptor::Map(
                Box::new(#inner_tokens)
            )
        });
    }

    Ok(quote! {
        <#ty as ::actrpc_core::action::action_descriptor::traits::DescribeValue>::describe_value()
    })
}
