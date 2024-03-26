mod inner;

use proc_macro::TokenStream;

#[proc_macro_derive(EnumNums)]
pub fn enum_nums(ts: TokenStream) -> TokenStream {
    inner::enum_nums_inner(ts)
}

#[proc_macro]
pub fn knowledge(ts: TokenStream) -> TokenStream {
    match inner::knowledge_inner(ts.into()) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
