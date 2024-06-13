//! Procedural macros for Hyperswitch
#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod constraint_graph;

use proc_macro::TokenStream;

/// Allows the user to specify a constraint graph using a custom DSL, thus eliminating
/// the overhead of creating a `ConstraintGraphBuilder` and writing the construction
/// code yourself.
///
/// ## Example
/// ```
/// constraint_graph! {
///     imports {
///         use hyperswitch_constraint_graph::cgraph_prelude;
///         // additional key and value imports
///     }
///
///     domain PaymentMethods(
///         "payment_methods",
///         "payment methods eligibility"
///     );
///
///     key type = DirKey;
///     value type = DirValue;
///
///     rule(PaymentMethods):
///         PaymentMethod = Card -> any CardType;
/// }
/// ```
#[proc_macro]
pub fn constraint_graph(ts: TokenStream) -> TokenStream {
    match constraint_graph::constraint_graph_inner(ts) {
        Ok(result_ts) => result_ts,
        Err(err) => err.into_compile_error().into(),
    }
}
