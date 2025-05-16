//! Constants that are used in the domain models.

use std::{collections::HashSet, sync::LazyLock};

pub static ROUTING_ENABLED_PAYMENT_METHODS: LazyLock<HashSet<common_enums::PaymentMethod>> =
    LazyLock::new(|| {
        let mut set = HashSet::new();
        set.insert(common_enums::PaymentMethod::BankTransfer);
        set.insert(common_enums::PaymentMethod::BankDebit);
        set.insert(common_enums::PaymentMethod::BankRedirect);
        set
    });

pub static ROUTING_ENABLED_PAYMENT_METHOD_TYPES: LazyLock<
    HashSet<common_enums::PaymentMethodType>,
> = LazyLock::new(|| {
    let mut set = HashSet::new();
    set.insert(common_enums::PaymentMethodType::GooglePay);
    set.insert(common_enums::PaymentMethodType::ApplePay);
    set.insert(common_enums::PaymentMethodType::Klarna);
    set.insert(common_enums::PaymentMethodType::Paypal);
    set.insert(common_enums::PaymentMethodType::SamsungPay);
    set
});

/// Length of the unique reference ID generated for connector mandate requests
pub const CONNECTOR_MANDATE_REQUEST_REFERENCE_ID_LENGTH: usize = 18;
