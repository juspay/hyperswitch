use crate::settings::{
    ConnectorFields, PaymentMethodType as PaymentMethodTypeInfo, PayoutRequiredFields,
    RequiredFieldFinal,
};
use api_models::{
    enums::{
        Connector, FieldType,
        PaymentMethod::{BankTransfer, Card},
        PaymentMethodType,
    },
    payment_methods::RequiredFieldInfo,
};
use std::collections::HashMap;

impl Default for PayoutRequiredFields {
    fn default() -> Self {
        Self(HashMap::from([
            (
                Card,
                PaymentMethodTypeInfo(HashMap::from([
                    // Adyen
                    get_adyen_fields(PaymentMethodType::Debit),
                ])),
            ),
            (
                BankTransfer,
                PaymentMethodTypeInfo(HashMap::from([
                    // Adyen
                    get_adyen_fields(PaymentMethodType::Sepa),
                ])),
            ),
        ]))
    }
}

fn get_adyen_fields(
    payment_method_type: PaymentMethodType,
) -> (PaymentMethodType, ConnectorFields) {
    match payment_method_type {
        PaymentMethodType::Debit => (
            payment_method_type,
            ConnectorFields {
                fields: HashMap::from([(Connector::Adyen, get_card_fields())]),
            },
        ),
        PaymentMethodType::Sepa => (
            payment_method_type,
            ConnectorFields {
                fields: HashMap::from([(Connector::Adyen, get_sepa_fields())]),
            },
        ),
        _ => (
            payment_method_type,
            ConnectorFields {
                fields: HashMap::new(),
            },
        ),
    }
}

fn get_card_fields() -> RequiredFieldFinal {
    RequiredFieldFinal {
        mandate: HashMap::new(),
        non_mandate: HashMap::new(),
        common: HashMap::from([
            (
                "payout_method_data.card.card_number".to_string(),
                RequiredFieldInfo {
                    required_field: "payout_method_data.card.card_number".to_string(),
                    display_name: "Card Number".to_string(),
                    field_type: FieldType::UserCardNumber,
                    value: None,
                },
            ),
            (
                "payout_method_data.card.expiry_month".to_string(),
                RequiredFieldInfo {
                    required_field: "payout_method_data.card.expiry_month".to_string(),
                    display_name: "Expiry Month".to_string(),
                    field_type: FieldType::UserCardExpiryMonth,
                    value: None,
                },
            ),
            (
                "payout_method_data.card.expiry_year".to_string(),
                RequiredFieldInfo {
                    required_field: "payout_method_data.card.expiry_year".to_string(),
                    display_name: "Expiry Year".to_string(),
                    field_type: FieldType::UserCardExpiryYear,
                    value: None,
                },
            ),
            (
                "payout_method_data.card.card_holder_name".to_string(),
                RequiredFieldInfo {
                    required_field: "payout_method_data.card.card_holder_name".to_string(),
                    display_name: "Cardholder Name".to_string(),
                    field_type: FieldType::UserFullName,
                    value: None,
                },
            ),
        ]),
    }
}

fn get_sepa_fields() -> RequiredFieldFinal {
    RequiredFieldFinal {
        mandate: HashMap::new(),
        non_mandate: HashMap::new(),
        common: HashMap::from([(
            "payout_method_data.bank.iban".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.iban".to_string(),
                display_name: "IBAN".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        )]),
    }
}
