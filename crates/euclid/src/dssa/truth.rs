use euclid_macros::knowledge;
use hyperswitch_macros::constraint_graph;
use once_cell::sync::Lazy;

use crate::{dssa::graph::euclid_graph_prelude, frontend::dir};

pub static ANALYSIS_GRAPH: Lazy<hyperswitch_constraint_graph::ConstraintGraph<'_, dir::DirValue>> =
    Lazy::new(|| {
        constraint_graph! {
            imports {
                use hyperswitch_constraint_graph::cgraph_prelude;
                use crate::frontend::dir::{enums::*,  DirKey, DirValue};
            }

            domain PaymentMethods(
                "payment_methods",
                "payment methods eligibility"
            );

            key type = DirKey;
            value type = DirValue;

            rule(PaymentMethods):
                PaymentMethod == Card -> any CardType;

            rule(PaymentMethods):
                PaymentMethod == PayLater -> any PayLaterType;

            rule(PaymentMethods):
                PaymentMethod == Wallet -> any WalletType;

            rule(PaymentMethods):
                PaymentMethod == BankRedirect -> any BankRedirectType;

            rule(PaymentMethods):
                PaymentMethod == BankTransfer -> any BankTransferType;

            rule(PaymentMethods):
                PaymentMethod == GiftCard -> any GiftCardType;
        }
    });
