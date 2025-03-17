use std::collections::HashMap;

use euclid_macros::costexpr;
use literally::hmap;
use once_cell::sync::Lazy;

use crate::{
    cost::{expr::CostExpr, prelude::*},
    enums::Connector,
};

pub static DEFAULTS: Lazy<HashMap<Connector, CostExpr>> = Lazy::new(|| {
    hmap! {
        Connector::Stripe => costexpr!(
            5 USD
        ),

        Connector::Braintree => costexpr!(
            branch PaymentMethod {
                Card => 49 mUSD + 2% * amount
                    + branch PaymentCurrency {
                        USD => 0 USD,
                        default => 1% * amount,
                    }
                    + branch BillingCountry {
                        UnitedStatesOfAmerica => 0 USD,
                        default => 1% * amount,
                    },

                BankDebit => branch BankDebitType {
                    Ach => 0.75% * amount,
                    default => 0 USD,
                },

                default => 0 USD,
            }
        ),
        Connector::Authorizedotnet => costexpr!(
        2.9% * amount + 30 mUSD
    ),
    }
});
