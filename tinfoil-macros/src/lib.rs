mod implementation;

#[proc_macro_derive(Tinfoil)]
pub fn tinfoil(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    crate::implementation::tinfoil(input).unwrap_or_else(|e| e)
}

#[proc_macro_derive(TinfoilContext, attributes(tinfoil))]
pub fn tinfoil_context(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    crate::implementation::tinfoil_context(input).unwrap_or_else(|e| e)
}
