mod describe_ok;
mod describe_params;
mod describe_value;
mod shared;

use proc_macro::TokenStream;

#[proc_macro_derive(DescribeValue)]
pub fn derive_describe_value(input: TokenStream) -> TokenStream {
    describe_value::expand(input)
}

#[proc_macro_derive(DescribeParams)]
pub fn derive_describe_params(input: TokenStream) -> TokenStream {
    describe_params::expand(input)
}

#[proc_macro_derive(DescribeOk)]
pub fn derive_describe_ok(input: TokenStream) -> TokenStream {
    describe_ok::expand(input)
}
