mod constraint_graph;

use proc_macro::TokenStream;

#[proc_macro]
pub fn constraint_graph(ts: TokenStream) -> TokenStream {
    match constraint_graph::constraint_graph_inner(ts) {
        Ok(result_ts) => result_ts,
        Err(err) => err.into_compile_error().into(),
    }
}
