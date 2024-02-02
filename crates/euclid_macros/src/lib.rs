mod inner;

use proc_macro::TokenStream;

#[proc_macro_derive(EnumNums)]
/// This function takes a TokenStream as input and passes it to the inner::enum_nums_inner function to process and return a new TokenStream.
pub fn enum_nums(ts: TokenStream) -> TokenStream {
    inner::enum_nums_inner(ts)
}

#[proc_macro]
/// This method takes a TokenStream as input and processes it using the knowledge_inner function from the inner module. It then matches the result, returning a new TokenStream if the inner function succeeded, or a compile error TokenStream if it failed.
pub fn knowledge(ts: TokenStream) -> TokenStream {
    match inner::knowledge_inner(ts.into()) {
        Ok(ts) => ts.into(),
        Err(e) => e.into_compile_error().into(),
    }
}
