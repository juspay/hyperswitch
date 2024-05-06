use constraint_graph_macros::constraint_graph;
use euclid_macros::knowledge;
use once_cell::sync::Lazy;

use crate::{dssa::graph::euclid_graph_prelude, frontend::dir};

pub static ANALYSIS_GRAPH: Lazy<hyperswitch_constraint_graph::ConstraintGraph<'_, dir::DirValue>> =
    Lazy::new(|| {
        constraint_graph! {
            imports {
                use hyperswitch_constraint_graph as cgraph;
                use crate::frontend::dir::{enums::*, DirKey, DirKeyKind, DirValue};
            }

            domain PaymentMethods
                with identifier "payment_methods"
                and description "payment methods eligibility";

            key type = DirKey;
            value type = DirValue;
        }
    });
