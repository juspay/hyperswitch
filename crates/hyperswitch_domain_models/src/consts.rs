//! Constants that are used in the domain models.

use std::collections::HashSet;

use router_env::once_cell::sync::Lazy;

#[cfg(all(any(feature = "v1", feature = "v2"), not(feature = "customer_v2")))]
pub const API_VERSION: common_enums::ApiVersion = common_enums::ApiVersion::V1;

#[cfg(all(feature = "v2", feature = "customer_v2"))]
pub const API_VERSION: common_enums::ApiVersion = common_enums::ApiVersion::V2;

pub static ROUTING_ENABLED_PAYMENT_METHODS: Lazy<HashSet<common_enums::PaymentMethod>> =
    Lazy::new(|| {
        let mut set = HashSet::new();
        set.insert(common_enums::PaymentMethod::BankTransfer);
        set.insert(common_enums::PaymentMethod::BankDebit);
        set.insert(common_enums::PaymentMethod::BankRedirect);
        set
    });

pub static ROUTING_ENABLED_PAYMENT_METHOD_TYPES: Lazy<HashSet<common_enums::PaymentMethodType>> =
    Lazy::new(|| {
        let mut set = HashSet::new();
        set.insert(common_enums::PaymentMethodType::GooglePay);
        set.insert(common_enums::PaymentMethodType::ApplePay);
        set.insert(common_enums::PaymentMethodType::Klarna);
        set.insert(common_enums::PaymentMethodType::Paypal);
        set.insert(common_enums::PaymentMethodType::SamsungPay);
        set
    });
