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
                    get_adyen_fields(PaymentMethodType::Credit),
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
    let mut common_fields = get_billing_details();
    match payment_method_type {
        PaymentMethodType::Debit => {
            common_fields.extend(get_card_fields());
            (
                payment_method_type,
                ConnectorFields {
                    fields: HashMap::from([(
                        Connector::Adyen,
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: common_fields,
                        },
                    )]),
                },
            )
        }
        PaymentMethodType::Credit => {
            common_fields.extend(get_card_fields());
            (
                payment_method_type,
                ConnectorFields {
                    fields: HashMap::from([(
                        Connector::Adyen,
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: common_fields,
                        },
                    )]),
                },
            )
        }
        PaymentMethodType::Sepa => {
            common_fields.extend(get_sepa_fields());
            (
                payment_method_type,
                ConnectorFields {
                    fields: HashMap::from([(
                        Connector::Adyen,
                        RequiredFieldFinal {
                            mandate: HashMap::new(),
                            non_mandate: HashMap::new(),
                            common: common_fields,
                        },
                    )]),
                },
            )
        }
        _ => (
            payment_method_type,
            ConnectorFields {
                fields: HashMap::new(),
            },
        ),
    }
}

fn get_card_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        (
            "payout_method_data.card.card_number".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.card.card_number".to_string(),
                display_name: "card_number".to_string(),
                field_type: FieldType::UserCardNumber,
                value: None,
            },
        ),
        (
            "payout_method_data.card.expiry_month".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.card.expiry_month".to_string(),
                display_name: "exp_month".to_string(),
                field_type: FieldType::UserCardExpiryMonth,
                value: None,
            },
        ),
        (
            "payout_method_data.card.expiry_year".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.card.expiry_year".to_string(),
                display_name: "exp_year".to_string(),
                field_type: FieldType::UserCardExpiryYear,
                value: None,
            },
        ),
        (
            "payout_method_data.card.card_holder_name".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.card.card_holder_name".to_string(),
                display_name: "card_holder_name".to_string(),
                field_type: FieldType::UserFullName,
                value: None,
            },
        ),
    ])
}

fn get_sepa_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        (
            "payout_method_data.bank.iban".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.iban".to_string(),
                display_name: "iban".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "payout_method_data.bank.bic".to_string(),
            RequiredFieldInfo {
                required_field: "payout_method_data.bank.bic".to_string(),
                display_name: "bic".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
    ])
}

fn get_billing_details() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        (
            "billing.address.city".to_string(),
            RequiredFieldInfo {
                required_field: "billing.address.city".to_string(),
                display_name: "billing_address_city".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "billing.address.country".to_string(),
            RequiredFieldInfo {
                required_field: "billing.address.country".to_string(),
                display_name: "billing_address_country".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "billing.address.line1".to_string(),
            RequiredFieldInfo {
                required_field: "billing.address.line1".to_string(),
                display_name: "billing_address_line1".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "billing.address.line2".to_string(),
            RequiredFieldInfo {
                required_field: "billing.address.line2".to_string(),
                display_name: "billing_address_line2".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "billing.address.zip".to_string(),
            RequiredFieldInfo {
                required_field: "billing.address.zip".to_string(),
                display_name: "billing_address_zip".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "billing.address.state".to_string(),
            RequiredFieldInfo {
                required_field: "billing.address.state".to_string(),
                display_name: "billing_address_state".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "billing.address.first_name".to_string(),
            RequiredFieldInfo {
                required_field: "billing.address.first_name".to_string(),
                display_name: "billing_address_first_name".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "billing.address.last_name".to_string(),
            RequiredFieldInfo {
                required_field: "billing.address.last_name".to_string(),
                display_name: "billing_address_last_name".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "billing.phone.number".to_string(),
            RequiredFieldInfo {
                required_field: "billing.phone.number".to_string(),
                display_name: "billing_phone_number".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
        (
            "billing.phone.country_code".to_string(),
            RequiredFieldInfo {
                required_field: "billing.phone.country_code".to_string(),
                display_name: "billing_phone_country_code".to_string(),
                field_type: FieldType::Text,
                value: None,
            },
        ),
    ])
}
