use euclid_macros::knowledge;
use once_cell::sync::Lazy;

use crate::{dssa::graph::euclid_graph_prelude, frontend::dir};

pub static ANALYSIS_GRAPH: Lazy<hyperswitch_constraint_graph::ConstraintGraph<'_, dir::DirValue>> =
    Lazy::new(|| {
        knowledge! {
            // Payment Method should be `Card` for a CardType to be present
            PaymentMethod(Card) ->> CardType(any);

            // Payment Method should be `PayLater` for a PayLaterType to be present
            PaymentMethod(PayLater) ->> PayLaterType(any);

            // Payment Method should be `Wallet` for a WalletType to be present
            PaymentMethod(Wallet) ->> WalletType(any);

            // Payment Method should be `BankRedirect` for a BankRedirectType to
            // be present
            PaymentMethod(BankRedirect) ->> BankRedirectType(any);

            // Payment Method should be `BankTransfer` for a BankTransferType to
            // be present
            PaymentMethod(BankTransfer) ->> BankTransferType(any);

            // Payment Method should be `GiftCard` for a GiftCardType to
            // be present
            PaymentMethod(GiftCard) ->> GiftCardType(any);
        }
    });
