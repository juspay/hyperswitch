use std::collections::{HashMap, HashSet};

use api_models::{enums, payment_methods::RequiredFieldInfo};

use crate::settings::{
    self, ConnectorFields, Mandates, PaymentMethodType, RequiredFieldFinal,
    SupportedConnectorsForMandate, SupportedPaymentMethodTypesForMandate,
    SupportedPaymentMethodsForMandate,
};

impl Default for Mandates {
    fn default() -> Self {
        Self {
            supported_payment_methods: SupportedPaymentMethodsForMandate(HashMap::from([
                (
                    enums::PaymentMethod::PayLater,
                    SupportedPaymentMethodTypesForMandate(HashMap::from([(
                        enums::PaymentMethodType::Klarna,
                        SupportedConnectorsForMandate {
                            connector_list: HashSet::from([enums::Connector::Adyen]),
                        },
                    )])),
                ),
                (
                    enums::PaymentMethod::Wallet,
                    SupportedPaymentMethodTypesForMandate(HashMap::from([
                        (
                            enums::PaymentMethodType::GooglePay,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([
                                    enums::Connector::Stripe,
                                    enums::Connector::Adyen,
                                    enums::Connector::Globalpay,
                                    enums::Connector::Multisafepay,
                                    enums::Connector::Bankofamerica,
                                    enums::Connector::Novalnet,
                                    enums::Connector::Noon,
                                    enums::Connector::Cybersource,
                                    enums::Connector::Wellsfargo,
                                ]),
                            },
                        ),
                        (
                            enums::PaymentMethodType::ApplePay,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([
                                    enums::Connector::Stripe,
                                    enums::Connector::Adyen,
                                    enums::Connector::Bankofamerica,
                                    enums::Connector::Cybersource,
                                    enums::Connector::Novalnet,
                                    enums::Connector::Wellsfargo,
                                ]),
                            },
                        ),
                        (
                            enums::PaymentMethodType::SamsungPay,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([enums::Connector::Cybersource]),
                            },
                        ),
                    ])),
                ),
                (
                    enums::PaymentMethod::Card,
                    SupportedPaymentMethodTypesForMandate(HashMap::from([
                        (
                            enums::PaymentMethodType::Credit,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([
                                    enums::Connector::Aci,
                                    enums::Connector::Adyen,
                                    enums::Connector::Authorizedotnet,
                                    enums::Connector::Globalpay,
                                    enums::Connector::Worldpay,
                                    enums::Connector::Fiuu,
                                    enums::Connector::Multisafepay,
                                    enums::Connector::Nexinets,
                                    enums::Connector::Noon,
                                    enums::Connector::Novalnet,
                                    enums::Connector::Payme,
                                    enums::Connector::Stripe,
                                    enums::Connector::Bankofamerica,
                                    enums::Connector::Cybersource,
                                    enums::Connector::Wellsfargo,
                                ]),
                            },
                        ),
                        (
                            enums::PaymentMethodType::Debit,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([
                                    enums::Connector::Aci,
                                    enums::Connector::Adyen,
                                    enums::Connector::Authorizedotnet,
                                    enums::Connector::Globalpay,
                                    enums::Connector::Worldpay,
                                    enums::Connector::Fiuu,
                                    enums::Connector::Multisafepay,
                                    enums::Connector::Nexinets,
                                    enums::Connector::Noon,
                                    enums::Connector::Novalnet,
                                    enums::Connector::Payme,
                                    enums::Connector::Stripe,
                                ]),
                            },
                        ),
                    ])),
                ),
            ])),
            update_mandate_supported: SupportedPaymentMethodsForMandate(HashMap::default()),
        }
    }
}

#[cfg(feature = "v1")]
impl Default for settings::RequiredFields {
    fn default() -> Self {
        Self(HashMap::from([
            (
                enums::PaymentMethod::Card,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::Debit,
                        ConnectorFields {
                        fields: HashMap::from([
                            (
                                enums::Connector::Aci,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Airwallex,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Authorizedotnet,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Bambora,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Bankofamerica,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Billwerk,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Bluesnap,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Boku,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Braintree,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Checkout,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Coinbase,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Cybersource,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common:HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Datatrans,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Deutschebank,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate : HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "first_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "last_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Dlocal,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                           (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                           (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common:HashMap::new(),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector1,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector2,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                               enums::Connector::DummyConnector3,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector4,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector5,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector6,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector7,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Elavon,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                            }
                                            ),

                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Fiserv,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Fiuu,
                                RequiredFieldFinal {
                                    mandate: HashMap::from([
                                        (
                                            "billing.email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "card_holder_name".to_string(),
                                                field_type: enums::FieldType::UserFullName,
                                                value: None,
                                            }
                                        ),
                                    ]),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Forte,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common:HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Globalpay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from([
                                        (
                                            "payment_method_data.card.card_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_number".to_string(),
                                                display_name: "card_number".to_string(),
                                                field_type: enums::FieldType::UserCardNumber,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_month".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                display_name: "card_exp_month".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryMonth,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_year".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                display_name: "card_exp_year".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryYear,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_cvc".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_cvc".to_string(),
                                                display_name: "card_cvc".to_string(),
                                                field_type: enums::FieldType::UserCardCvc,
                                                value: None,
                                            }
                                        )
                                    ]),
                                }
                            ),
                            (
                                enums::Connector::Helcim,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Iatapay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Mollie,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Moneris,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Multisafepay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate:HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line2".to_string(),
                                                    display_name: "line2".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine2,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Nexinets,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Nexixpay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line2".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine2,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "first_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "last_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Nmi,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "billing_zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Noon,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Novalnet,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "first_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "last_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email_address".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Nuvei,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Paybox,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )

                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Payme,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Paypal,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Payu,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Powertranz,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Rapyd,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Shift4,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Square,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Stax,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Stripe,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common:HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Trustpay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new()
                                }
                            ),
                            (
                                enums::Connector::Tsys,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new()
                                }
                            ),
                            (
                                enums::Connector::Wellsfargo,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common:HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Worldline,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "payment_method_data.card.card_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_number".to_string(),
                                                display_name: "card_number".to_string(),
                                                field_type: enums::FieldType::UserCardNumber,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_month".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                display_name: "card_exp_month".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryMonth,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_year".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                display_name: "card_exp_year".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryYear,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_cvc".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_cvc".to_string(),
                                                display_name: "card_cvc".to_string(),
                                                field_type: enums::FieldType::UserCardCvc,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "country".to_string(),
                                                field_type: enums::FieldType::UserAddressCountry{
                                                    options: vec![
                                                        "ALL".to_string(),
                                                    ]
                                                },
                                                value: None,
                                            }
                                        )
                                    ]),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Worldpay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: {
                                        let mut pmd_fields = HashMap::from([
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            )
                                        ]);
                                        pmd_fields.extend(get_worldpay_billing_required_fields());
                                        pmd_fields
                                    },
                                }
                            ),
                            (
                                enums::Connector::Xendit,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate:HashMap::new(),
                                    common:  HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                            }
                                            ),
                                            (
                                                "payment_method_data.billing.phone.number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.phone.number".to_string(),
                                                    display_name: "phone_number".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumber,
                                                    value: None,
                                                }
                                            )

                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Zen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "payment_method_data.card.card_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_number".to_string(),
                                                display_name: "card_number".to_string(),
                                                field_type: enums::FieldType::UserCardNumber,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_month".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                display_name: "card_exp_month".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryMonth,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_year".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                display_name: "card_exp_year".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryYear,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                            }
                                        ),
                                    ]),
                                    common: HashMap::new(),
                                }
                            ),
                        ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Credit,
                        ConnectorFields {
                        fields: HashMap::from([
                            (
                                enums::Connector::Aci,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Airwallex,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Authorizedotnet,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Bambora,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Bankofamerica,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Billwerk,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Bluesnap,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Boku,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Braintree,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Checkout,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Coinbase,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                   display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Cybersource,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate:HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Datatrans,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Deutschebank,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate : HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "first_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "last_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Dlocal,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common:HashMap::new(),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector1,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector2,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector3,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector4,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector5,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector6,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            #[cfg(feature = "dummy_connector")]
                            (
                                enums::Connector::DummyConnector7,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Elavon,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                            }
                                            ),

                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Fiserv,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Fiuu,
                                RequiredFieldFinal {
                                    mandate: HashMap::from([
                                        (
                                            "billing.email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "card_holder_name".to_string(),
                                                field_type: enums::FieldType::UserFullName,
                                                value: None,
                                            }
                                        ),
                                    ]),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Forte,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common:HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Globalpay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from([
                                        (
                                            "payment_method_data.card.card_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_number".to_string(),
                                                display_name: "card_number".to_string(),
                                                field_type: enums::FieldType::UserCardNumber,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_month".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                display_name: "card_exp_month".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryMonth,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_year".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                display_name: "card_exp_year".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryYear,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_cvc".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_cvc".to_string(),
                                                display_name: "card_cvc".to_string(),
                                                field_type: enums::FieldType::UserCardCvc,
                                                value: None,
                                            }
                                        )
                                    ]),
                                }
                            ),
                            (
                                enums::Connector::Helcim,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Iatapay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Mollie,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Moneris,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Multisafepay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate:HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line2".to_string(),
                                                    display_name: "line2".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine2,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Nexinets,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Nexixpay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line2".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine2,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "first_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "last_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Nmi,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                   display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "billing_zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Noon,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Novalnet,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "first_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "last_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email_address".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Nuvei,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),(
                                enums::Connector::Paybox,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )

                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Payme,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                     display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Paypal,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Payu,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Powertranz,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Rapyd,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Shift4,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Square,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Stax,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Stripe,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common:HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Trustpay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                        ]
                                    ),
                                    common: HashMap::new()
                                }
                            ),
                            (
                                enums::Connector::Tsys,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new()
                                }
                            ),
                            (
                                enums::Connector::Wellsfargo,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate:HashMap::new(),
                                    common: HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Worldline,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "payment_method_data.card.card_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_number".to_string(),
                                                display_name: "card_number".to_string(),
                                                field_type: enums::FieldType::UserCardNumber,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_month".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                display_name: "card_exp_month".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryMonth,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_year".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                display_name: "card_exp_year".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryYear,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_cvc".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_cvc".to_string(),
                                                display_name: "card_cvc".to_string(),
                                                field_type: enums::FieldType::UserCardCvc,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "country".to_string(),
                                                field_type: enums::FieldType::UserAddressCountry{
                                                    options: vec![
                                                        "ALL".to_string(),
                                                    ]
                                                },
                                                value: None,
                                            }
                                        )
                                    ]),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Worldpay,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: {
                                        let mut pmd_fields = HashMap::from([
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            )
                                        ]);
                                        pmd_fields.extend(get_worldpay_billing_required_fields());
                                        pmd_fields
                                    },
                                }
                            ),
                            (
                                enums::Connector::Xendit,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate:HashMap::new(),
                                    common:  HashMap::from(
                                        [
                                            (
                                                "payment_method_data.card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.card.card_cvc".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_cvc".to_string(),
                                                    display_name: "card_cvc".to_string(),
                                                    field_type: enums::FieldType::UserCardCvc,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                            }
                                            ),
                                            (
                                                "payment_method_data.billing.phone.number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.phone.number".to_string(),
                                                    display_name: "phone_number".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumber,
                                                    value: None,
                                                }
                                            )

                                        ]
                                    ),
                                }
                            ),
                            (
                                enums::Connector::Zen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "payment_method_data.card.card_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_number".to_string(),
                                                display_name: "card_number".to_string(),
                                                field_type: enums::FieldType::UserCardNumber,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_month".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_month".to_string(),
                                                display_name: "card_exp_month".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryMonth,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.card.card_exp_year".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.card.card_exp_year".to_string(),
                                                display_name: "card_exp_year".to_string(),
                                                field_type: enums::FieldType::UserCardExpiryYear,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                            }
                                        ),
                                    ]),
                                    common: HashMap::new(),
                                }
                            ),
                        ]),
                        },
                    ),

                ])),
            ),
            (
                enums::PaymentMethod::BankRedirect,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::OpenBankingUk,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                enums::Connector::Volt,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "billing_first_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.last_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                display_name: "billing_last_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                    ]),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap:: from([
                                        (
                                            "payment_method_data.bank_redirect.open_banking_uk.issuer".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.open_banking_uk.issuer".to_string(),
                                                display_name: "issuer".to_string(),
                                                field_type: enums::FieldType::UserBank,
                                                value: None,
                                            }
                                        )
                                    ]),
                                    common: HashMap::new(),
                                }
                            )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Trustly,
                        ConnectorFields {
                            fields: HashMap::from([
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::new(),
                                }
                            )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::OnlineBankingCzechRepublic,
                        ConnectorFields {
                            fields: HashMap::from([
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "payment_method_data.bank_redirect.open_banking_czech_republic.issuer".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.open_banking_czech_republic.issuer".to_string(),
                                                display_name: "issuer".to_string(),
                                                field_type: enums::FieldType::UserBank,
                                                value: None,
                                            }
                                        )
                            ]),
                                    common: HashMap::new(),
                                }
                            )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::OnlineBankingFinland,
                        ConnectorFields {
                            fields: HashMap::from([
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "billing.email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                            }
                                        ),
                            ]),
                                    common: HashMap::new(),
                                }
                            )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::OnlineBankingPoland,
                        ConnectorFields {
                            fields: HashMap::from([
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "payment_method_data.bank_redirect.open_banking_poland.issuer".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.open_banking_poland.issuer".to_string(),
                                                display_name: "issuer".to_string(),
                                                field_type: enums::FieldType::UserBank,
                                                value: None,
                                            }
                                        ),

                                        (
                                            "billing.email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                            }
                                        ),
                            ]),
                                    common: HashMap::new(),
                                }
                            )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::OnlineBankingSlovakia,
                        ConnectorFields {
                            fields: HashMap::from([
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "payment_method_data.bank_redirect.open_banking_slovakia.issuer".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.open_banking_slovakia.issuer".to_string(),
                                                display_name: "issuer".to_string(),
                                                field_type: enums::FieldType::UserBank,
                                                value: None,
                                            }
                                        ),
                            ]),
                                    common: HashMap::new(),
                                }
                            )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::OnlineBankingFpx,
                        ConnectorFields {
                            fields: HashMap::from([
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "payment_method_data.bank_redirect.open_banking_fpx.issuer".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.open_banking_fpx.issuer".to_string(),
                                                display_name: "issuer".to_string(),
                                                field_type: enums::FieldType::UserBank,
                                                value: None,
                                            }
                                        )
                            ]),
                                    common: HashMap::new(),
                                }
                            )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::OnlineBankingThailand,
                        ConnectorFields {
                            fields: HashMap::from([
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "payment_method_data.bank_redirect.open_banking_thailand.issuer".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.open_banking_thailand.issuer".to_string(),
                                                display_name: "issuer".to_string(),
                                                field_type: enums::FieldType::UserBank,
                                                value: None,
                                            }
                                        )
                            ]),
                                    common: HashMap::new(),
                                }
                            )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Bizum,
                        ConnectorFields {
                            fields: HashMap::from([
                            (
                                enums::Connector::Adyen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::new(),
                                }
                            )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Przelewy24,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                enums::Connector::Stripe,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::from([
                                        (
                                            "billing.email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                            }
                                        )
                                    ]),
                                    common: HashMap::new(),
                                }
                            )]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::BancontactCard,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Mollie,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::from([
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "payment_method_data.bank_redirect.bancontact_card.card_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.bancontact_card.card_number".to_string(),
                                                    display_name: "card_number".to_string(),
                                                    field_type: enums::FieldType::UserCardNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_redirect.bancontact_card.card_exp_month".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.bancontact_card.card_exp_month".to_string(),
                                                    display_name: "card_exp_month".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryMonth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_redirect.bancontact_card.card_exp_year".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.bancontact_card.card_exp_year".to_string(),
                                                    display_name: "card_exp_year".to_string(),
                                                    field_type: enums::FieldType::UserCardExpiryYear,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Giropay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Aci,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                               "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserCountry {
                                                        options: vec![
                                                                "DE".to_string(),
                                                        ]},
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Globalpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            ("billing.address.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "country".to_string(),
                                                field_type: enums::FieldType::UserAddressCountry {
                                                    options: vec![
                                                            "DE".to_string(),
                                                        ]
                                                },
                                                value: None,
                                            }
                                        )
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Mollie,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Nuvei,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate:HashMap::from([
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "DE".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )]
                                        ),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Paypal,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.address.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "country".to_string(),
                                                field_type: enums::FieldType::UserCountry {
                                                    options: vec![
                                                            "DE".to_string(),
                                                        ]
                                                },
                                                value: None,
                                            }
                                        ),
                                        (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Shift4,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Trustpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry {
                                                        options: vec![
                                                                "DE".to_string(),
                                                            ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Ideal,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Aci,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "payment_method_data.bank_redirect.ideal.bank_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.ideal.bank_name".to_string(),
                                                    display_name: "bank_name".to_string(),
                                                    field_type: enums::FieldType::UserBank,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                 "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserCountry {
                                                        options: vec![
                                                            "NL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "payment_method_data.bank_redirect.ideal.bank_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.ideal.bank_name".to_string(),
                                                    display_name: "bank_name".to_string(),
                                                    field_type: enums::FieldType::UserBank,
                                                    value: None,
                                                }
                                            ),

                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Globalpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Mollie,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Nexinets,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Nuvei,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "NL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Shift4,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                 "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserCountry{
                                                        options: vec![
                                                            "NL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Paypal,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                             (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserCountry{
                                                        options: vec![
                                                            "NL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "billing_email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        non_mandate: HashMap::new(),
                                        common:  HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Trustpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "NL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Sofort,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Aci,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            ("billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "country".to_string(),
                                                field_type: enums::FieldType::UserCountry {
                                                    options: vec![
                                                            "ES".to_string(),
                                                            "GB".to_string(),
                                                            "SE".to_string(),
                                                            "AT".to_string(),
                                                            "NL".to_string(),
                                                            "DE".to_string(),
                                                            "CH".to_string(),
                                                            "BE".to_string(),
                                                            "FR".to_string(),
                                                            "FI".to_string(),
                                                            "IT".to_string(),
                                                            "PL".to_string(),
                                                        ]
                                                },
                                                value: None,
                                            }
                                        )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Globalpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            ("billing.address.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "country".to_string(),
                                                field_type: enums::FieldType::UserAddressCountry {
                                                    options: vec![
                                                            "AT".to_string(),
                                                            "BE".to_string(),
                                                            "DE".to_string(),
                                                            "ES".to_string(),
                                                            "IT".to_string(),
                                                            "NL".to_string(),
                                                        ]
                                                },
                                                value: None,
                                            }
                                        )
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Mollie,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Nexinets,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Nuvei,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate:HashMap::from([
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ES".to_string(),
                                                            "GB".to_string(),
                                                            "IT".to_string(),
                                                            "DE".to_string(),
                                                            "FR".to_string(),
                                                            "AT".to_string(),
                                                            "BE".to_string(),
                                                            "NL".to_string(),
                                                            "BE".to_string(),
                                                            "SK".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )]
                                        ),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Paypal,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            ( "billing.address.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "country".to_string(),
                                                field_type: enums::FieldType::UserCountry {
                                                    options: vec![
                                                            "ES".to_string(),
                                                            "GB".to_string(),
                                                            "AT".to_string(),
                                                            "NL".to_string(),
                                                            "DE".to_string(),
                                                            "BE".to_string(),
                                                        ]
                                                },
                                                value: None,
                                            }
                                        ),
                                         (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Shift4,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::from([
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        non_mandate : HashMap::new(),
                                        common: HashMap::from([
                                            ("billing.address.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "country".to_string(),
                                                field_type: enums::FieldType::UserCountry {
                                                    options: vec![
                                                            "ES".to_string(),
                                                            "AT".to_string(),
                                                            "NL".to_string(),
                                                            "DE".to_string(),
                                                            "BE".to_string(),
                                                        ]
                                                },
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "account_holder_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.last_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                display_name: "account_holder_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        )]),
                                    }
                                ),
                                (
                                    enums::Connector::Trustpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry {
                                                        options: vec![
                                                                "ES".to_string(),
                                                                "GB".to_string(),
                                                                "SE".to_string(),
                                                                "AT".to_string(),
                                                                "NL".to_string(),
                                                                "DE".to_string(),
                                                                "CH".to_string(),
                                                                "BE".to_string(),
                                                                "FR".to_string(),
                                                                "FI".to_string(),
                                                                "IT".to_string(),
                                                                "PL".to_string(),
                                                            ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Eps,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "payment_method_data.bank_redirect.eps.bank_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.eps.bank_name".to_string(),
                                                    display_name: "bank_name".to_string(),
                                                    field_type: enums::FieldType::UserBank,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Aci,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "bank_account_country".to_string(),
                                                    field_type: enums::FieldType::UserCountry {
                                                        options: vec![
                                                            "AT".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Globalpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common:  HashMap::from([
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry {
                                                        options: vec![
                                                            "AT".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ])
                                    }
                                ),
                                (
                                    enums::Connector::Mollie,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate:HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Paypal,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "bank_account_country".to_string(),
                                                    field_type: enums::FieldType::UserCountry {
                                                        options: vec![
                                                            "AT".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Trustpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "AT".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Shift4,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate:HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Nuvei,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "AT".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )]
                                        ),
                                        common: HashMap::new(),
                                    }
                                ),
                                ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Blik,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "payment_method_data.bank_redirect.blik.blik_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.blik.blik_code".to_string(),
                                                    display_name: "blik_code".to_string(),
                                                    field_type: enums::FieldType::UserBlikCode,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "payment_method_data.bank_redirect.blik.blik_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.blik.blik_code".to_string(),
                                                    display_name: "blik_code".to_string(),
                                                    field_type: enums::FieldType::UserBlikCode,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Trustpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                    }
                                )
                                ]),
                        },
                    ),
                ])),
            ),
            (
                enums::PaymentMethod::Wallet,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::ApplePay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Bankofamerica,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from(
                                            [
                                                (
                                                    "email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                        display_name: "billing_first_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "billing_last_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                            options: vec![
                                                                "ALL".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                )
                                            ]
                                        ),
                                    }
                                ),
                                (
                                    enums::Connector::Cybersource,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from(
                                            [
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                        display_name: "billing_first_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "billing_last_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                            options: vec![
                                                                "ALL".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                )
                                            ]
                                        ),
                                    }
                                ),
                                (
                                    enums::Connector::Novalnet,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from(
                                            [
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email_address".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                            ]
                                        ),
                                    }
                                ),
                                (
                                    enums::Connector::Wellsfargo,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from(
                                            [
                                                (
                                                    "email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                        display_name: "billing_first_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "billing_last_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                            options: vec![
                                                                "ALL".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.first_name".to_string(),
                                                        display_name: "shipping_first_name".to_string(),
                                                        field_type: enums::FieldType::UserShippingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.last_name".to_string(),
                                                        display_name: "shipping_last_name".to_string(),
                                                        field_type: enums::FieldType::UserShippingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressCountry{
                                                            options: vec![
                                                                "ALL".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                            ]
                                        ),
                                        common: HashMap::new(),
                                    }
                                ),

                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::SamsungPay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Cybersource,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::GooglePay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Bankofamerica,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common:  HashMap::from(
                                            [
                                                (
                                                    "email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                        display_name: "billing_first_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "billing_last_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                            options: vec![
                                                                "ALL".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                )
                                            ]
                                        ),
                                    }
                                ),
                                (
                                    enums::Connector::Bluesnap,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Noon,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Novalnet,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from(
                                            [
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email_address".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                            ]
                                        ),
                                    }
                                ),
                                (
                                    enums::Connector::Nuvei,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Airwallex,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Authorizedotnet,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Checkout,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Globalpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Multisafepay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from([
                                        (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "billing_first_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.last_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                display_name: "billing_last_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.city".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.city".to_string(),
                                                display_name: "city".to_string(),
                                                field_type: enums::FieldType::UserAddressCity,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.state".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.state".to_string(),
                                                display_name: "state".to_string(),
                                                field_type: enums::FieldType::UserAddressState,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.zip".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.zip".to_string(),
                                                display_name: "zip".to_string(),
                                                field_type: enums::FieldType::UserAddressPincode,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "country".to_string(),
                                                field_type: enums::FieldType::UserAddressCountry{
                                                    options: vec![
                                                        "ALL".to_string(),
                                                    ]
                                                },
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.line1".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.line1".to_string(),
                                                display_name: "line1".to_string(),
                                                field_type: enums::FieldType::UserAddressLine1,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.line2".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.line2".to_string(),
                                                display_name: "line2".to_string(),
                                                field_type: enums::FieldType::UserAddressLine2,
                                                value: None,
                                            }
                                        )]),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Cybersource,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from(
                                            [
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                        display_name: "billing_first_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "billing_last_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                            options: vec![
                                                                "ALL".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                )
                                            ]
                                        ),
                                    }
                                ),
                                (
                                    enums::Connector::Payu,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Rapyd,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Trustpay,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Wellsfargo,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from(
                                            [
                                                (
                                                    "email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                        display_name: "billing_first_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "billing_last_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                            options: vec![
                                                                "ALL".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.first_name".to_string(),
                                                        display_name: "shipping_first_name".to_string(),
                                                        field_type: enums::FieldType::UserShippingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.last_name".to_string(),
                                                        display_name: "shipping_last_name".to_string(),
                                                        field_type: enums::FieldType::UserShippingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressCountry{
                                                            options: vec![
                                                                "ALL".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                            ]
                                        ),
                                        common: HashMap::new(),
                                    }
                                ),
                               ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::WeChatPay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::AliPay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::AliPayHk,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::AmazonPay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Cashapp,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::MbWay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        common: HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "payment_method_data.billing.phone.number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.phone.number".to_string(),
                                                    display_name: "phone_number".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.country_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                    display_name: "dialing_code".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                    value: None,
                                                }
                                            ),
                                            ]
                                        ),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::KakaoPay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Twint,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Gcash,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Vipps,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Dana,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Momo,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Swish,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::TouchNGo,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        // Added shipping fields for the SDK flow to accept it from wallet directly,
                        // this won't show up in SDK in payment's sheet but will be used in the background
                        enums::PaymentMethodType::Paypal,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            )]
                                        ),
                                    }
                                ),
                                (
                                    enums::Connector::Braintree,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Novalnet,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from(
                                            [
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email_address".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                            ]
                                        ),
                                    }
                                ),
                                (
                                    enums::Connector::Paypal,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(
                                        ),
                                        common: HashMap::from(
                                            [
                                                (
                                                    "shipping.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.first_name".to_string(),
                                                        display_name: "shipping_first_name".to_string(),
                                                        field_type: enums::FieldType::UserShippingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.last_name".to_string(),
                                                        display_name: "shipping_last_name".to_string(),
                                                        field_type: enums::FieldType::UserShippingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressCountry{
                                                            options: vec![
                                                                "ALL".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                            ]
                                        ),
                                    }
                                ),
                               ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Mifinity,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Mifinity,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "payment_method_data.wallet.mifinity.date_of_birth".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.wallet.mifinity.date_of_birth".to_string(),
                                                    display_name: "date_of_birth".to_string(),
                                                    field_type: enums::FieldType::UserDateOfBirth,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "first_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "last_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.number".to_string(),
                                                    display_name: "phone".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumber,
                                                    value: None,
                                                }
                                            ),
                                             (
                                                "billing.phone.country_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                    display_name: "dialing_code".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "nationality".to_string(),
                                                    field_type: enums::FieldType::UserCountry{
                                                        options: vec![
                                                                "BR".to_string(),
                                                                "CN".to_string(),
                                                                "SG".to_string(),
                                                                "MY".to_string(),
                                                                "DE".to_string(),
                                                                "CH".to_string(),
                                                                "DK".to_string(),
                                                                "GB".to_string(),
                                                                "ES".to_string(),
                                                                "AD".to_string(),
                                                                "GI".to_string(),
                                                                "FI".to_string(),
                                                                "FR".to_string(),
                                                                "GR".to_string(),
                                                                "HR".to_string(),
                                                                "IT".to_string(),
                                                                "JP".to_string(),
                                                                "MX".to_string(),
                                                                "AR".to_string(),
                                                                "CO".to_string(),
                                                                "CL".to_string(),
                                                                "PE".to_string(),
                                                                "VE".to_string(),
                                                                "UY".to_string(),
                                                                "PY".to_string(),
                                                                "BO".to_string(),
                                                                "EC".to_string(),
                                                                "GT".to_string(),
                                                                "HN".to_string(),
                                                                "SV".to_string(),
                                                                "NI".to_string(),
                                                                "CR".to_string(),
                                                                "PA".to_string(),
                                                                "DO".to_string(),
                                                                "CU".to_string(),
                                                                "PR".to_string(),
                                                                "NL".to_string(),
                                                                "NO".to_string(),
                                                                "PL".to_string(),
                                                                "PT".to_string(),
                                                                "SE".to_string(),
                                                                "RU".to_string(),
                                                                "TR".to_string(),
                                                                "TW".to_string(),
                                                                "HK".to_string(),
                                                                "MO".to_string(),
                                                                "AX".to_string(),
                                                                "AL".to_string(),
                                                                "DZ".to_string(),
                                                                "AS".to_string(),
                                                                "AO".to_string(),
                                                                "AI".to_string(),
                                                                "AG".to_string(),
                                                                "AM".to_string(),
                                                                "AW".to_string(),
                                                                "AU".to_string(),
                                                                "AT".to_string(),
                                                                "AZ".to_string(),
                                                                "BS".to_string(),
                                                                "BH".to_string(),
                                                                "BD".to_string(),
                                                                "BB".to_string(),
                                                                "BE".to_string(),
                                                                "BZ".to_string(),
                                                                "BJ".to_string(),
                                                                "BM".to_string(),
                                                                "BT".to_string(),
                                                                "BQ".to_string(),
                                                                "BA".to_string(),
                                                                "BW".to_string(),
                                                                "IO".to_string(),
                                                                "BN".to_string(),
                                                                "BG".to_string(),
                                                                "BF".to_string(),
                                                                "BI".to_string(),
                                                                "KH".to_string(),
                                                                "CM".to_string(),
                                                                "CA".to_string(),
                                                                "CV".to_string(),
                                                                "KY".to_string(),
                                                                "CF".to_string(),
                                                                "TD".to_string(),
                                                                "CX".to_string(),
                                                                "CC".to_string(),
                                                                "KM".to_string(),
                                                                "CG".to_string(),
                                                                "CK".to_string(),
                                                                "CI".to_string(),
                                                                "CW".to_string(),
                                                                "CY".to_string(),
                                                                "CZ".to_string(),
                                                                "DJ".to_string(),
                                                                "DM".to_string(),
                                                                "EG".to_string(),
                                                                "GQ".to_string(),
                                                                "ER".to_string(),
                                                                "EE".to_string(),
                                                                "ET".to_string(),
                                                                "FK".to_string(),
                                                                "FO".to_string(),
                                                                "FJ".to_string(),
                                                                "GF".to_string(),
                                                                "PF".to_string(),
                                                                "TF".to_string(),
                                                                "GA".to_string(),
                                                                "GM".to_string(),
                                                                "GE".to_string(),
                                                                "GH".to_string(),
                                                                "GL".to_string(),
                                                                "GD".to_string(),
                                                                "GP".to_string(),
                                                                "GU".to_string(),
                                                                "GG".to_string(),
                                                                "GN".to_string(),
                                                                "GW".to_string(),
                                                                "GY".to_string(),
                                                                "HT".to_string(),
                                                                "HM".to_string(),
                                                                "VA".to_string(),
                                                                "IS".to_string(),
                                                                "IN".to_string(),
                                                                "ID".to_string(),
                                                                "IE".to_string(),
                                                                "IM".to_string(),
                                                                "IL".to_string(),
                                                                "JE".to_string(),
                                                                "JO".to_string(),
                                                                "KZ".to_string(),
                                                                "KE".to_string(),
                                                                "KI".to_string(),
                                                                "KW".to_string(),
                                                                "KG".to_string(),
                                                                "LA".to_string(),
                                                                "LV".to_string(),
                                                                "LB".to_string(),
                                                                "LS".to_string(),
                                                                "LI".to_string(),
                                                                "LT".to_string(),
                                                                "LU".to_string(),
                                                                "MK".to_string(),
                                                                "MG".to_string(),
                                                                "MW".to_string(),
                                                                "MV".to_string(),
                                                                "ML".to_string(),
                                                                "MT".to_string(),
                                                                "MH".to_string(),
                                                                "MQ".to_string(),
                                                                "MR".to_string(),
                                                                "MU".to_string(),
                                                                "YT".to_string(),
                                                                "FM".to_string(),
                                                                "MD".to_string(),
                                                                "MC".to_string(),
                                                                "MN".to_string(),
                                                                "ME".to_string(),
                                                                "MS".to_string(),
                                                                "MA".to_string(),
                                                                "MZ".to_string(),
                                                                "NA".to_string(),
                                                                "NR".to_string(),
                                                                "NP".to_string(),
                                                                "NC".to_string(),
                                                                "NZ".to_string(),
                                                                "NE".to_string(),
                                                                "NG".to_string(),
                                                                "NU".to_string(),
                                                                "NF".to_string(),
                                                                "MP".to_string(),
                                                                "OM".to_string(),
                                                                "PK".to_string(),
                                                                "PW".to_string(),
                                                                "PS".to_string(),
                                                                "PG".to_string(),
                                                                "PH".to_string(),
                                                                "PN".to_string(),
                                                                "QA".to_string(),
                                                                "RE".to_string(),
                                                                "RO".to_string(),
                                                                "RW".to_string(),
                                                                "BL".to_string(),
                                                                "SH".to_string(),
                                                                "KN".to_string(),
                                                                "LC".to_string(),
                                                                "MF".to_string(),
                                                                "PM".to_string(),
                                                                "VC".to_string(),
                                                                "WS".to_string(),
                                                                "SM".to_string(),
                                                                "ST".to_string(),
                                                                "SA".to_string(),
                                                                "SN".to_string(),
                                                                "RS".to_string(),
                                                                "SC".to_string(),
                                                                "SL".to_string(),
                                                                "SX".to_string(),
                                                                "SK".to_string(),
                                                                "SI".to_string(),
                                                                "SB".to_string(),
                                                                "SO".to_string(),
                                                                "ZA".to_string(),
                                                                "GS".to_string(),
                                                                "KR".to_string(),
                                                                "LK".to_string(),
                                                                "SR".to_string(),
                                                                "SJ".to_string(),
                                                                "SZ".to_string(),
                                                                "TH".to_string(),
                                                                "TL".to_string(),
                                                                "TG".to_string(),
                                                                "TK".to_string(),
                                                                "TO".to_string(),
                                                                "TT".to_string(),
                                                                "TN".to_string(),
                                                                "TM".to_string(),
                                                                "TC".to_string(),
                                                                "TV".to_string(),
                                                                "UG".to_string(),
                                                                "UA".to_string(),
                                                                "AE".to_string(),
                                                                "UZ".to_string(),
                                                                "VU".to_string(),
                                                                "VN".to_string(),
                                                                "VG".to_string(),
                                                                "VI".to_string(),
                                                                "WF".to_string(),
                                                                "EH".to_string(),
                                                                "ZM".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }

                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email_address".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.wallet.mifinity.language_preference".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.wallet.mifinity.language_preference".to_string(),
                                                    display_name: "language_preference".to_string(),
                                                    field_type: enums::FieldType::LanguagePreference{
                                                        options: vec![
                                                            "BR".to_string(),
                                                            "PT_BR".to_string(),
                                                            "CN".to_string(),
                                                            "ZH_CN".to_string(),
                                                            "DE".to_string(),
                                                            "DK".to_string(),
                                                            "DA".to_string(),
                                                            "DA_DK".to_string(),
                                                            "EN".to_string(),
                                                            "ES".to_string(),
                                                            "FI".to_string(),
                                                            "FR".to_string(),
                                                            "GR".to_string(),
                                                            "EL".to_string(),
                                                            "EL_GR".to_string(),
                                                            "HR".to_string(),
                                                            "IT".to_string(),
                                                            "JP".to_string(),
                                                            "JA".to_string(),
                                                            "JA_JP".to_string(),
                                                            "LA".to_string(),
                                                            "ES_LA".to_string(),
                                                            "NL".to_string(),
                                                            "NO".to_string(),
                                                            "PL".to_string(),
                                                            "PT".to_string(),
                                                            "RU".to_string(),
                                                            "SV".to_string(),
                                                            "SE".to_string(),
                                                            "SV_SE".to_string(),
                                                            "ZH".to_string(),
                                                            "TW".to_string(),
                                                            "ZH_TW".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                    }
                                ),
                            ]),
                        }
                    ),
                ])),
            ),
            (
                enums::PaymentMethod::PayLater,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::AfterpayClearpay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                    options: vec![
                                                        "GB".to_string(),
                                                        "AU".to_string(),
                                                        "CA".to_string(),
                                                        "US".to_string(),
                                                        "NZ".to_string(),
                                                    ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.first_name".to_string(),
                                                    display_name: "shipping_first_name".to_string(),
                                                    field_type: enums::FieldType::UserShippingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.last_name".to_string(),
                                                    display_name: "shipping_last_name".to_string(),
                                                    field_type: enums::FieldType::UserShippingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserShippingAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserShippingAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserShippingAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserShippingAddressCountry{
                                                        options: vec![
                                                            "ALL".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserShippingAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                        common : HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line2".to_string(),
                                                    display_name: "line2".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine2,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                    options: vec![
                                                        "GB".to_string(),
                                                        "AU".to_string(),
                                                        "CA".to_string(),
                                                        "US".to_string(),
                                                        "NZ".to_string(),
                                                    ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserShippingAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserShippingAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserShippingAddressCountry{
                                                        options: vec![
                                                        "GB".to_string(),
                                                        "AU".to_string(),
                                                        "CA".to_string(),
                                                        "US".to_string(),
                                                        "NZ".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserShippingAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "shipping.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "shipping.address.line2".to_string(),
                                                    display_name: "line2".to_string(),
                                                    field_type: enums::FieldType::UserShippingAddressLine2,
                                                    value: None,
                                                }
                                            ),
                                    ]),
                                    common : HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Klarna,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate: HashMap::from([
                                            ( "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "billing_country".to_string(),
                                                field_type: enums::FieldType::UserAddressCountry{
                                                    options:  vec![
                                                        "AU".to_string(),
                                                        "AT".to_string(),
                                                        "BE".to_string(),
                                                        "CA".to_string(),
                                                        "CZ".to_string(),
                                                        "DK".to_string(),
                                                        "FI".to_string(),
                                                        "FR".to_string(),
                                                        "GR".to_string(),
                                                        "DE".to_string(),
                                                        "IE".to_string(),
                                                        "IT".to_string(),
                                                        "NL".to_string(),
                                                        "NZ".to_string(),
                                                        "NO".to_string(),
                                                        "PL".to_string(),
                                                        "PT".to_string(),
                                                        "RO".to_string(),
                                                        "ES".to_string(),
                                                        "SE".to_string(),
                                                        "CH".to_string(),
                                                        "GB".to_string(),
                                                        "US".to_string(),
                                                    ]
                                                },
                                                value: None,
                                            }),
                                            ("billing.email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                           })
                                        ]),
                                        common : HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common : HashMap::from([
                                            ( "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "billing_country".to_string(),
                                                field_type: enums::FieldType::UserAddressCountry{
                                                    options: vec![
                                                        "ALL".to_string(),
                                                    ]
                                                },
                                                value: None,
                                            }),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Klarna,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate:  HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "AU".to_string(),
                                                            "AT".to_string(),
                                                            "BE".to_string(),
                                                            "CA".to_string(),
                                                            "CZ".to_string(),
                                                            "DK".to_string(),
                                                            "FI".to_string(),
                                                            "FR".to_string(),
                                                            "DE".to_string(),
                                                            "GR".to_string(),
                                                            "IE".to_string(),
                                                            "IT".to_string(),
                                                            "NL".to_string(),
                                                            "NZ".to_string(),
                                                            "NO".to_string(),
                                                            "PL".to_string(),
                                                            "PT".to_string(),
                                                            "ES".to_string(),
                                                            "SE".to_string(),
                                                            "CH".to_string(),
                                                            "GB".to_string(),
                                                            "US".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            )
                                        ]),
                                    }
                                )
                                ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Affirm,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from(
                                            [
                                                (
                                                    "billing.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                        display_name: "card_holder_name".to_string(),
                                                        field_type: enums::FieldType::UserFullName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "card_holder_name".to_string(),
                                                        field_type: enums::FieldType::UserFullName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                            options: vec![
                                                                "US".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.phone.number".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.phone.number".to_string(),
                                                        display_name: "phone_number".to_string(),
                                                        field_type: enums::FieldType::UserPhoneNumber,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.phone.country_code".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                        display_name: "dialing_code".to_string(),
                                                        field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line2".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line2".to_string(),
                                                        display_name: "line2".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine2,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.line2".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.line2".to_string(),
                                                        display_name: "line2".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine2,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserCountry {
                                                            options: vec![
                                                                    "US".to_string(),
                                                            ]},
                                                        value: None,
                                                    }
                                                ),

                                            ]
                                        ),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::PayBright,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from(
                                            [
                                                (
                                                    "billing.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                        display_name: "card_holder_name".to_string(),
                                                        field_type: enums::FieldType::UserFullName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "card_holder_name".to_string(),
                                                        field_type: enums::FieldType::UserFullName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                            options: vec![
                                                                "CA".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "payment_method_data.billing.phone.number".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.phone.number".to_string(),
                                                        display_name: "phone_number".to_string(),
                                                        field_type: enums::FieldType::UserPhoneNumber,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.phone.country_code".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                        display_name: "dialing_code".to_string(),
                                                        field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line2".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line2".to_string(),
                                                        display_name: "line2".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine2,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressCountry{
                                                            options: vec![
                                                                "ALL".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "shipping.address.line2".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "shipping.address.line2".to_string(),
                                                        display_name: "line2".to_string(),
                                                        field_type: enums::FieldType::UserShippingAddressLine2,
                                                        value: None,
                                                    }
                                                ),
                                            ]
                                        ),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Walley,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from(
                                            [
                                                (
                                                    "billing.phone.number".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.phone.number".to_string(),
                                                        display_name: "phone".to_string(),
                                                        field_type: enums::FieldType::UserPhoneNumber,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                                "DK".to_string(),
                                                                "FI".to_string(),
                                                                "NO".to_string(),
                                                                "SE".to_string(),
                                                            ]},
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.phone.country_code".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                        display_name: "dialing_code".to_string(),
                                                        field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                            ]
                                        ),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Alma,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from(
                                            [
                                                (
                                                    "billing.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                        display_name: "card_holder_name".to_string(),
                                                        field_type: enums::FieldType::UserFullName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "card_holder_name".to_string(),
                                                        field_type: enums::FieldType::UserFullName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                            options: vec![
                                                                "FR".to_string(),
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "payment_method_data.billing.phone.number".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "billing.phone.number".to_string(),
                                                        display_name: "phone_number".to_string(),
                                                        field_type: enums::FieldType::UserPhoneNumber,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.phone.country_code".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                        display_name: "dialing_code".to_string(),
                                                        field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line2".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line2".to_string(),
                                                        display_name: "line2".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine2,
                                                        value: None,
                                                    }
                                                )
                                            ]
                                        ),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Atome,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::from(
                                            [
                                                (
                                                    "billing.address.first_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                        display_name: "card_holder_name".to_string(),
                                                        field_type: enums::FieldType::UserFullName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "card_holder_name".to_string(),
                                                        field_type: enums::FieldType::UserFullName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.city".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.city".to_string(),
                                                        display_name: "city".to_string(),
                                                        field_type: enums::FieldType::UserAddressCity,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.state".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.state".to_string(),
                                                        display_name: "state".to_string(),
                                                        field_type: enums::FieldType::UserAddressState,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.zip".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.zip".to_string(),
                                                        display_name: "zip".to_string(),
                                                        field_type: enums::FieldType::UserAddressPincode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.country".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.country".to_string(),
                                                        display_name: "country".to_string(),
                                                        field_type: enums::FieldType::UserAddressCountry{
                                                            options: vec![
                                                                "MY".to_string(),
                                                                "SG".to_string()
                                                            ]
                                                        },
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "payment_method_data.billing.phone.number".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "billing.phone.number".to_string(),
                                                        display_name: "phone_number".to_string(),
                                                        field_type: enums::FieldType::UserPhoneNumber,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.phone.country_code".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                        display_name: "dialing_code".to_string(),
                                                        field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "billing.address.line2".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.line2".to_string(),
                                                        display_name: "line2".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine2,
                                                        value: None,
                                                    }
                                                )
                                            ]
                                        ),
                                        common: HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                ])),
            ),
            (
                enums::PaymentMethod::Crypto,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::CryptoCurrency,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Cryptopay,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate: HashMap::from([
                                            (
                                                "payment_method_data.crypto.pay_currency".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.crypto.pay_currency".to_string(),
                                                    display_name: "currency".to_string(),
                                                    field_type: enums::FieldType::UserCurrency{
                                                        options: vec![
                                                            "BTC".to_string(),
                                                            "LTC".to_string(),
                                                            "ETH".to_string(),
                                                            "XRP".to_string(),
                                                            "XLM".to_string(),
                                                            "BCH".to_string(),
                                                            "ADA".to_string(),
                                                            "SOL".to_string(),
                                                            "SHIB".to_string(),
                                                            "TRX".to_string(),
                                                            "DOGE".to_string(),
                                                            "BNB".to_string(),
                                                            "USDT".to_string(),
                                                            "USDC".to_string(),
                                                            "DAI".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.crypto.network".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.crypto.network".to_string(),
                                                    display_name: "network".to_string(),
                                                    field_type: enums::FieldType::UserCryptoCurrencyNetwork,
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                        common : HashMap::new(),
                                    }
                                ),
                            ]),
                        },
                    ),
                ])),
            ),
            (
                enums::PaymentMethod::Voucher,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::Boleto,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate : HashMap::from([
                                            (
                                                "payment_method_data.voucher.boleto.social_security_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.voucher.boleto.social_security_number".to_string(),
                                                    display_name: "social_security_number".to_string(),
                                                    field_type: enums::FieldType::Text,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "BR".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.line2".to_string(),
                                                    display_name: "line2".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine2,
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                        common : HashMap::new(),
                                    }
                                ),
                                (
                                    enums::Connector::Zen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Alfamart,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate : HashMap::from([
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common : HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Indomaret,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate : HashMap::from([
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        common : HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Oxxo,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate : HashMap::new(),
                                        common : HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::SevenEleven,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate : HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.number".to_string(),
                                                    display_name: "phone".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.country_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                    display_name: "dialing_code".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                    value: None,
                                                }
                                            )
                                            ]
                                        ),
                                        common : HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Lawson,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate : HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.number".to_string(),
                                                    display_name: "phone".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.country_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                    display_name: "dialing_code".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                    value: None,
                                                }
                                            ),
                                            ]
                                        ),
                                        common : HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::MiniStop,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate : HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.number".to_string(),
                                                    display_name: "phone".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.country_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                    display_name: "dialing_code".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                    value: None,
                                                }
                                            ),
                                            ]
                                        ),
                                        common : HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::FamilyMart,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate : HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.number".to_string(),
                                                    display_name: "phone".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.country_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                    display_name: "dialing_code".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                    value: None,
                                                }
                                            ),
                                            ]
                                        ),
                                        common : HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Seicomart,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate : HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.number".to_string(),
                                                    display_name: "phone".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.country_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                    display_name: "dialing_code".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                    value: None,
                                                }
                                            ),
                                            ]
                                        ),
                                        common : HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::PayEasy,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate : HashMap::from([
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.number".to_string(),
                                                    display_name: "phone".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.phone.country_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                    display_name: "dialing_code".to_string(),
                                                    field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                    value: None,
                                                }
                                            ),
                                            ]
                                        ),
                                        common : HashMap::new(),
                                    }
                                )
                            ]),
                        },
                    ),
                ])),
            ),
            (
                enums::PaymentMethod::Upi,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::UpiCollect,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Razorpay,
                                    RequiredFieldFinal {
                                        mandate : HashMap::new(),
                                        non_mandate :  HashMap::new(),
                                        common : HashMap::from([
                                            (
                                                "payment_method_data.upi.upi_collect.vpa_id".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.upi.upi_collect.vpa_id".to_string(),
                                                    display_name: "vpa_id".to_string(),
                                                    field_type: enums::FieldType::UserVpaId,
                                                    value: None,
                                                }
                                            ),
                                        ]),
                                    }
                                ),
                            ]),
                        },
                    ),
                ])),
            ),
            (
                enums::PaymentMethod::BankDebit,
                PaymentMethodType(HashMap::from([(
                    enums::PaymentMethodType::Ach,
                    ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([(
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "billing_first_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.last_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                display_name: "owner_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.bank_debit.ach_bank_debit.account_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_debit.ach_bank_debit.account_number".to_string(),
                                                display_name: "bank_account_number".to_string(),
                                                field_type: enums::FieldType::UserBankAccountNumber,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.bank_debit.ach_bank_debit.routing_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_debit.ach_bank_debit.routing_number".to_string(),
                                                display_name: "bank_routing_number".to_string(),
                                                field_type: enums::FieldType::UserBankRoutingNumber,
                                                value: None,
                                            }
                                        )
                                        ]),
                                    }),
                                    (
                                        enums::Connector::Adyen,
                                        RequiredFieldFinal {
                                            mandate: HashMap::new(),
                                            non_mandate: HashMap::new(),
                                            common: HashMap::from([ (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "owner_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }),
                                                (
                                                    "billing.address.last_name".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                        display_name: "owner_name".to_string(),
                                                        field_type: enums::FieldType::UserBillingName,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "payment_method_data.bank_debit.ach_bank_debit.account_number".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.bank_debit.ach_bank_debit.account_number".to_string(),
                                                        display_name: "bank_account_number".to_string(),
                                                        field_type: enums::FieldType::UserBankAccountNumber,
                                                        value: None,
                                                    }
                                                ),
                                                (
                                                    "payment_method_data.bank_debit.ach_bank_debit.routing_number".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.bank_debit.ach_bank_debit.routing_number".to_string(),
                                                        display_name: "bank_routing_number".to_string(),
                                                        field_type: enums::FieldType::UserBankRoutingNumber,
                                                        value: None,
                                                    }
                                                )
                                            ]),
                                        })
                                    ]
                                )}
                    ),
                (
                        enums::PaymentMethodType::Sepa,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([ (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "billing_first_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.last_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                display_name: "owner_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.bank_debit.sepa_bank_debit.iban".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_debit.sepa_bank_debit.iban".to_string(),
                                                display_name: "iban".to_string(),
                                                field_type: enums::FieldType::UserIban,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                            }
                                        )
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([ (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "owner_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "owner_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_debit.sepa_bank_debit.iban".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_debit.sepa_bank_debit.iban".to_string(),
                                                    display_name: "iban".to_string(),
                                                    field_type: enums::FieldType::UserIban,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Deutschebank,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([ (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "owner_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "owner_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_debit.sepa_bank_debit.iban".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_debit.sepa_bank_debit.iban".to_string(),
                                                    display_name: "iban".to_string(),
                                                    field_type: enums::FieldType::UserIban,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                    }
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Bacs,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([ (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "billing_first_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.bank_debit.bacs_bank_debit.account_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_debit.bacs_bank_debit.account_number".to_string(),
                                                display_name: "bank_account_number".to_string(),
                                                field_type: enums::FieldType::UserBankAccountNumber,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.bank_debit.bacs_bank_debit.sort_code".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_debit.bacs_bank_debit.sort_code".to_string(),
                                                display_name: "bank_sort_code".to_string(),
                                                field_type: enums::FieldType::UserBankSortCode,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                display_name: "country".to_string(),
                                                field_type: enums::FieldType::UserAddressCountry {
                                                    options: vec!["UK".to_string()],
                                                },
                                                value: None,
                                            },
                                        ),
                                        (
                                            "billing.address.zip".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.zip".to_string(),
                                                display_name: "zip".to_string(),
                                                field_type: enums::FieldType::UserAddressPincode,
                                                value: None,
                                            },
                                        ),
                                        (
                                            "billing.address.line1".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.line1".to_string(),
                                                display_name: "line1".to_string(),
                                                field_type: enums::FieldType::UserAddressLine1,
                                                value: None,
                                            },
                                        )
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([ (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "owner_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "owner_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_debit.bacs_bank_debit.account_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_debit.bacs_bank_debit.account_number".to_string(),
                                                    display_name: "bank_account_number".to_string(),
                                                    field_type: enums::FieldType::UserBankAccountNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_debit.bacs_bank_debit.sort_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_debit.bacs_bank_debit.sort_code".to_string(),
                                                    display_name: "bank_sort_code".to_string(),
                                                    field_type: enums::FieldType::UserBankSortCode,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                    })
                                ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Becs,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([ (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "billing_first_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.last_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                display_name: "owner_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.bank_debit.becs_bank_debit.account_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_debit.becs_bank_debit.account_number".to_string(),
                                                display_name: "bank_account_number".to_string(),
                                                field_type: enums::FieldType::UserBankAccountNumber,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "payment_method_data.bank_debit.becs_bank_debit.bsb_number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_debit.becs_bank_debit.bsb_number".to_string(),
                                                display_name: "bsb_number".to_string(),
                                                field_type: enums::FieldType::UserBsbNumber,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                            }
                                        )
                                        ]),
                                    }
                                ),
                                (
                                    enums::Connector::Adyen,
                                    RequiredFieldFinal {
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([ (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                display_name: "owner_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                    display_name: "owner_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_debit.becs_bank_debit.account_number".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_debit.becs_bank_debit.account_number".to_string(),
                                                    display_name: "bank_account_number".to_string(),
                                                    field_type: enums::FieldType::UserBankAccountNumber,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_debit.becs_bank_debit.sort_code".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_debit.becs_bank_debit.sort_code".to_string(),
                                                    display_name: "bank_sort_code".to_string(),
                                                    field_type: enums::FieldType::UserBankSortCode,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                    })
                                ]),
                        },
                    ),
                    ]))),
                    (
                        enums::PaymentMethod::BankTransfer,
                        PaymentMethodType(HashMap::from([(
                            enums::PaymentMethodType::Multibanco,
                            ConnectorFields {
                                fields: HashMap::from([
                                    (
                                        enums::Connector::Stripe,
                                        RequiredFieldFinal {
                                            mandate: HashMap::new(),
                                            non_mandate: HashMap::from([
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                )
                                            ]),
                                            common: HashMap::new(),
                                        }
                                    ),
                                ])}),
                                (enums::PaymentMethodType::LocalBankTransfer,
                            ConnectorFields {
                                fields: HashMap::from([
                                    (
                                        enums::Connector::Zsl,
                                        RequiredFieldFinal {
                                            mandate: HashMap::new(),
                                            non_mandate: HashMap::from([ (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.country".to_string(),
                                                    display_name: "country".to_string(),
                                                    field_type: enums::FieldType::UserAddressCountry{
                                                        options: vec![
                                                            "CN".to_string(),
                                                        ]
                                                    },
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                },
                                            ),
                                            ]),
                                            common: HashMap::new(),
                                        }
                                    ),
                                ])}),
                                (enums::PaymentMethodType::Ach,
                            ConnectorFields {
                                fields: HashMap::from([
                                    (
                                        enums::Connector::Stripe,
                                        RequiredFieldFinal {
                                            mandate: HashMap::new(),
                                            non_mandate: HashMap::new(),
                                            common: HashMap::from([
                                                (
                                                    "billing.email".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "payment_method_data.billing.email".to_string(),
                                                        display_name: "email".to_string(),
                                                        field_type: enums::FieldType::UserEmailAddress,
                                                        value: None,
                                                    }
                                                )
                                            ])
                                        }
                                    ),
                                ])}),
                                (enums::PaymentMethodType::Pix,
                            ConnectorFields {
                                fields: HashMap::from([
                                    (
                                        enums::Connector::Itaubank,
                                        RequiredFieldFinal {
                                            mandate: HashMap::new(),
                                            non_mandate: HashMap::new(),
                                            common: HashMap::from(
                                                [
                                                    (
                                                        "payment_method_data.bank_transfer.pix.pix_key".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.bank_transfer.pix.pix_key".to_string(),
                                                            display_name: "pix_key".to_string(),
                                                            field_type: enums::FieldType::UserPixKey,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "payment_method_data.bank_transfer.pix.cnpj".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.bank_transfer.pix.cnpj".to_string(),
                                                            display_name: "cnpj".to_string(),
                                                            field_type: enums::FieldType::UserCnpj,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "payment_method_data.bank_transfer.pix.cpf".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.bank_transfer.pix.cpf".to_string(),
                                                            display_name: "cpf".to_string(),
                                                            field_type: enums::FieldType::UserCpf,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "billing.address.first_name".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                            display_name: "card_holder_name".to_string(),
                                                            field_type: enums::FieldType::UserFullName,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "billing.address.last_name".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                            display_name: "card_holder_name".to_string(),
                                                            field_type: enums::FieldType::UserFullName,
                                                            value: None,
                                                        }
                                                    ),
                                                ]
                                            ),
                                        }
                                    ),
                                    (
                                        enums::Connector::Adyen,
                                        RequiredFieldFinal {
                                            mandate: HashMap::new(),
                                            non_mandate: HashMap::new(),
                                            common: HashMap::new(),
                                        }
                                    ),
                                ])}),
                                (
                                    enums::PaymentMethodType::PermataBankTransfer,
                                    ConnectorFields {
                                        fields: HashMap::from([
                                            (
                                                enums::Connector::Adyen,
                                                RequiredFieldFinal {
                                                    mandate : HashMap::new(),
                                                    non_mandate : HashMap::from([
                                                        (
                                                            "billing.email".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.email".to_string(),
                                                                display_name: "email".to_string(),
                                                                field_type: enums::FieldType::UserEmailAddress,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.first_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.last_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        )
                                                    ]),
                                                    common : HashMap::new(),
                                                }
                                            )
                                        ]),
                                    },
                                ),
                                (
                                    enums::PaymentMethodType::BcaBankTransfer,
                                    ConnectorFields {
                                        fields: HashMap::from([
                                            (
                                                enums::Connector::Adyen,
                                                RequiredFieldFinal {
                                                    mandate : HashMap::new(),
                                                    non_mandate : HashMap::from([
                                                        (
                                                            "billing.email".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.email".to_string(),
                                                                display_name: "email".to_string(),
                                                                field_type: enums::FieldType::UserEmailAddress,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.first_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.last_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        )
                                                    ]),
                                                    common : HashMap::new(),
                                                }
                                            )
                                        ]),
                                    },
                                ),
                                (
                                    enums::PaymentMethodType::BniVa,
                                    ConnectorFields {
                                        fields: HashMap::from([
                                            (
                                                enums::Connector::Adyen,
                                                RequiredFieldFinal {
                                                    mandate : HashMap::new(),
                                                    non_mandate : HashMap::from([
                                                        (
                                                            "billing.email".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.email".to_string(),
                                                                display_name: "email".to_string(),
                                                                field_type: enums::FieldType::UserEmailAddress,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.first_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.last_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        )
                                                    ]),
                                                    common : HashMap::new(),
                                                }
                                            )
                                        ]),
                                    },
                                ),
                                (
                                    enums::PaymentMethodType::BriVa,
                                    ConnectorFields {
                                        fields: HashMap::from([
                                            (
                                                enums::Connector::Adyen,
                                                RequiredFieldFinal {
                                                    mandate : HashMap::new(),
                                                    non_mandate : HashMap::from([
                                                        (
                                                            "billing.email".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.email".to_string(),
                                                                display_name: "email".to_string(),
                                                                field_type: enums::FieldType::UserEmailAddress,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.first_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.last_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        )
                                                    ]),
                                                    common : HashMap::new(),
                                                }
                                            )
                                        ]),
                                    },
                                ),
                                (
                                    enums::PaymentMethodType::CimbVa,
                                    ConnectorFields {
                                        fields: HashMap::from([
                                            (
                                                enums::Connector::Adyen,
                                                RequiredFieldFinal {
                                                    mandate : HashMap::new(),
                                                    non_mandate : HashMap::from([
                                                        (
                                                            "billing.email".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.email".to_string(),
                                                                display_name: "email".to_string(),
                                                                field_type: enums::FieldType::UserEmailAddress,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.first_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.last_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        )
                                                    ]),
                                                    common : HashMap::new(),
                                                }
                                            )
                                        ]),
                                    },
                                ),
                                (
                                    enums::PaymentMethodType::DanamonVa,
                                    ConnectorFields {
                                        fields: HashMap::from([
                                            (
                                                enums::Connector::Adyen,
                                                RequiredFieldFinal {
                                                    mandate : HashMap::new(),
                                                    non_mandate : HashMap::from([
                                                        (
                                                            "billing.email".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.email".to_string(),
                                                                display_name: "email".to_string(),
                                                                field_type: enums::FieldType::UserEmailAddress,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.first_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.last_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        )
                                                    ]),
                                                    common : HashMap::new(),
                                                }
                                            )
                                        ]),
                                    },
                                ),
                                (
                                    enums::PaymentMethodType::MandiriVa,
                                    ConnectorFields {
                                        fields: HashMap::from([
                                            (
                                                enums::Connector::Adyen,
                                                RequiredFieldFinal {
                                                    mandate : HashMap::new(),
                                                    non_mandate : HashMap::from([
                                                        (
                                                            "billing.email".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.email".to_string(),
                                                                display_name: "email".to_string(),
                                                                field_type: enums::FieldType::UserEmailAddress,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.first_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.last_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        )
                                                    ]),
                                                    common : HashMap::new(),
                                                }
                                            )
                                        ]),
                                    },
                                ),
                                (
                                    enums::PaymentMethodType::Sepa,
                                    ConnectorFields {
                                        fields: HashMap::from([
                                            (
                                                enums::Connector::Stripe,
                                                RequiredFieldFinal {
                                                    mandate : HashMap::new(),
                                                    non_mandate : HashMap::new(),
                                                    common : HashMap::from([
                                                        (
                                                            "billing.email".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.email".to_string(),
                                                                display_name: "email".to_string(),
                                                                field_type: enums::FieldType::UserEmailAddress,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.first_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.last_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.country".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.country".to_string(),
                                                                display_name: "country".to_string(),
                                                                field_type: enums::FieldType::UserAddressCountry {
                                                                    options: vec![
                                                                        "BE".to_string(),
                                                                        "DE".to_string(),
                                                                        "ES".to_string(),
                                                                        "FR".to_string(),
                                                                        "IE".to_string(),
                                                                        "NL".to_string(),
                                                                    ],
                                                                },
                                                                value: None,
                                                            },
                                                        ),
                                                    ]),
                                                }
                                            )
                                        ]),
                                    },
                                ),
                                (
                                    enums::PaymentMethodType::Bacs,
                                    ConnectorFields {
                                        fields: HashMap::from([
                                            (
                                                enums::Connector::Stripe,
                                                RequiredFieldFinal {
                                                    mandate : HashMap::new(),
                                                    non_mandate : HashMap::new(),
                                                    common : HashMap::from([
                                                        (
                                                            "billing.email".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.email".to_string(),
                                                                display_name: "email".to_string(),
                                                                field_type: enums::FieldType::UserEmailAddress,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.first_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "billing.address.last_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                                display_name: "card_holder_name".to_string(),
                                                                field_type: enums::FieldType::UserFullName,
                                                                value: None,
                                                            }
                                                        )
                                                    ]),
                                                }
                                            )
                                        ]),
                                    },
                                ),
                    ]))),
                    (
                        enums::PaymentMethod::GiftCard,
                        PaymentMethodType(HashMap::from([
                            (
                                enums::PaymentMethodType::PaySafeCard,
                                ConnectorFields {
                                    fields: HashMap::from([
                                        (
                                            enums::Connector::Adyen,
                                            RequiredFieldFinal {
                                                mandate: HashMap::new(),
                                                non_mandate: HashMap::new(),
                                                common: HashMap::new(),
                                            }
                                        ),
                                        ]),
                                },
                            ),
                            (
                                enums::PaymentMethodType::Givex,
                                ConnectorFields {
                                    fields: HashMap::from([
                                        (
                                            enums::Connector::Adyen,
                                            RequiredFieldFinal {
                                                mandate: HashMap::new(),
                                                non_mandate: HashMap::from([

                                                    (
                                                        "payment_method_data.gift_card.number".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.gift_card.number".to_string(),
                                                            display_name: "gift_card_number".to_string(),
                                                            field_type: enums::FieldType::UserCardNumber,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "payment_method_data.gift_card.cvc".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.gift_card.cvc".to_string(),
                                                            display_name: "gift_card_cvc".to_string(),
                                                            field_type: enums::FieldType::UserCardCvc,
                                                            value: None,
                                                        }
                                                    ),
                                                ]),
                                                common: HashMap::new(),
                                            }
                                        ),
                                        ]),
                                },
                            ),
                        ]))
                    ),
                    (
                        enums::PaymentMethod::CardRedirect,
                        PaymentMethodType(HashMap::from([
                            (
                                enums::PaymentMethodType::Benefit,
                                ConnectorFields {
                                    fields: HashMap::from([
                                        (
                                            enums::Connector::Adyen,
                                            RequiredFieldFinal {
                                                mandate: HashMap::new(),
                                                non_mandate: HashMap::from(
                                                    [(
                                                        "billing.address.first_name".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                            display_name: "first_name".to_string(),
                                                            field_type: enums::FieldType::UserFullName,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "billing.address.last_name".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                            display_name: "last_name".to_string(),
                                                            field_type: enums::FieldType::UserFullName,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "billing.phone.number".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.phone.number".to_string(),
                                                            display_name: "phone".to_string(),
                                                            field_type: enums::FieldType::UserPhoneNumber,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "billing.phone.country_code".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                            display_name: "dialing_code".to_string(),
                                                            field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "billing.email".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.email".to_string(),
                                                            display_name: "email".to_string(),
                                                            field_type: enums::FieldType::UserEmailAddress,
                                                            value: None,
                                                        }
                                                    )
                                                    ]
                                                ),
                                                common: HashMap::new(),
                                            }
                                        ),
                                        ]),
                                },
                            ),
                            (
                                enums::PaymentMethodType::Knet,
                                ConnectorFields {
                                    fields: HashMap::from([
                                        (
                                            enums::Connector::Adyen,
                                            RequiredFieldFinal {
                                                mandate: HashMap::new(),
                                                non_mandate: HashMap::from(
                                                    [(
                                                        "billing.address.first_name".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                            display_name: "first_name".to_string(),
                                                            field_type: enums::FieldType::UserFullName,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "billing.address.last_name".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.address.last_name".to_string(),
                                                            display_name: "last_name".to_string(),
                                                            field_type: enums::FieldType::UserFullName,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "billing.phone.number".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.phone.number".to_string(),
                                                            display_name: "phone".to_string(),
                                                            field_type: enums::FieldType::UserPhoneNumber,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "billing.phone.country_code".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.phone.country_code".to_string(),
                                                            display_name: "dialing_code".to_string(),
                                                            field_type: enums::FieldType::UserPhoneNumberCountryCode,
                                                            value: None,
                                                        }
                                                    ),
                                                    (
                                                        "billing.email".to_string(),
                                                        RequiredFieldInfo {
                                                            required_field: "payment_method_data.billing.email".to_string(),
                                                            display_name: "email".to_string(),
                                                            field_type: enums::FieldType::UserEmailAddress,
                                                            value: None,
                                                        }
                                                    )
                                                    ]
                                                ),
                                                common: HashMap::new(),
                                            }
                                        ),
                                        ]),
                                },
                            ),
                            (
                                enums::PaymentMethodType::MomoAtm,
                                ConnectorFields {
                                    fields: HashMap::from([
                                        (
                                            enums::Connector::Adyen,
                                            RequiredFieldFinal {
                                                mandate: HashMap::new(),
                                                non_mandate: HashMap::new(),
                                                common: HashMap::new(),
                                            }
                                        ),
                                        ]),
                                },
                            )
                        ]))
                    ),
                    (
                        enums::PaymentMethod::MobilePayment,
                        PaymentMethodType(HashMap::from([
                            (
                                enums::PaymentMethodType::DirectCarrierBilling,
                                ConnectorFields {
                                    fields: HashMap::from([
                                        (
                                            enums::Connector::Digitalvirgo,
                                            RequiredFieldFinal {
                                                mandate: HashMap::new(),
                                                non_mandate: HashMap::new(),
                                                common: HashMap::from(
                                                    [
                                                        (
                                                            "payment_method_data.mobile_payment.direct_carrier_billing.msisdn".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.mobile_payment.direct_carrier_billing.msisdn".to_string(),
                                                                display_name: "mobile_number".to_string(),
                                                                field_type: enums::FieldType::UserMsisdn,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "payment_method_data.mobile_payment.direct_carrier_billing.client_uid".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "payment_method_data.mobile_payment.direct_carrier_billing.client_uid".to_string(),
                                                                display_name: "client_identifier".to_string(),
                                                                field_type: enums::FieldType::UserClientIdentifier,
                                                                value: None,
                                                            }
                                                        ),
                                                        (
                                                            "order_details.0.product_name".to_string(),
                                                            RequiredFieldInfo {
                                                                required_field: "order_details.0.product_name".to_string(),
                                                                display_name: "product_name".to_string(),
                                                                field_type: enums::FieldType::OrderDetailsProductName,
                                                                value: None,
                                                            }
                                                        ),
                                                    ]
                                                ),
                                            }
                                        ),
                                    ])
                                }
                            )
                        ]))
                    )
        ]))
    }
}

pub fn get_worldpay_billing_required_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        (
            "billing.address.line1".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.address.line1".to_string(),
                display_name: "line1".to_string(),
                field_type: enums::FieldType::UserAddressLine1,
                value: None,
            },
        ),
        (
            "billing.address.country".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.address.country".to_string(),
                display_name: "country".to_string(),
                field_type: enums::FieldType::UserAddressCountry {
                    options: vec![
                        "AF".to_string(),
                        "AU".to_string(),
                        "AW".to_string(),
                        "AZ".to_string(),
                        "BS".to_string(),
                        "BH".to_string(),
                        "BD".to_string(),
                        "BB".to_string(),
                        "BZ".to_string(),
                        "BM".to_string(),
                        "BT".to_string(),
                        "BO".to_string(),
                        "BA".to_string(),
                        "BW".to_string(),
                        "BR".to_string(),
                        "BN".to_string(),
                        "BG".to_string(),
                        "BI".to_string(),
                        "KH".to_string(),
                        "CA".to_string(),
                        "CV".to_string(),
                        "KY".to_string(),
                        "CL".to_string(),
                        "CO".to_string(),
                        "KM".to_string(),
                        "CD".to_string(),
                        "CR".to_string(),
                        "CZ".to_string(),
                        "DZ".to_string(),
                        "DK".to_string(),
                        "DJ".to_string(),
                        "ST".to_string(),
                        "DO".to_string(),
                        "EC".to_string(),
                        "EG".to_string(),
                        "SV".to_string(),
                        "ER".to_string(),
                        "ET".to_string(),
                        "FK".to_string(),
                        "FJ".to_string(),
                        "GM".to_string(),
                        "GE".to_string(),
                        "GH".to_string(),
                        "GI".to_string(),
                        "GT".to_string(),
                        "GN".to_string(),
                        "GY".to_string(),
                        "HT".to_string(),
                        "HN".to_string(),
                        "HK".to_string(),
                        "HU".to_string(),
                        "IS".to_string(),
                        "IN".to_string(),
                        "ID".to_string(),
                        "IR".to_string(),
                        "IQ".to_string(),
                        "IE".to_string(),
                        "IL".to_string(),
                        "IT".to_string(),
                        "JM".to_string(),
                        "JP".to_string(),
                        "JO".to_string(),
                        "KZ".to_string(),
                        "KE".to_string(),
                        "KW".to_string(),
                        "LA".to_string(),
                        "LB".to_string(),
                        "LS".to_string(),
                        "LR".to_string(),
                        "LY".to_string(),
                        "LT".to_string(),
                        "MO".to_string(),
                        "MK".to_string(),
                        "MG".to_string(),
                        "MW".to_string(),
                        "MY".to_string(),
                        "MV".to_string(),
                        "MR".to_string(),
                        "MU".to_string(),
                        "MX".to_string(),
                        "MD".to_string(),
                        "MN".to_string(),
                        "MA".to_string(),
                        "MZ".to_string(),
                        "MM".to_string(),
                        "NA".to_string(),
                        "NZ".to_string(),
                        "NI".to_string(),
                        "NG".to_string(),
                        "KP".to_string(),
                        "NO".to_string(),
                        "AR".to_string(),
                        "PK".to_string(),
                        "PG".to_string(),
                        "PY".to_string(),
                        "PE".to_string(),
                        "UY".to_string(),
                        "PH".to_string(),
                        "PL".to_string(),
                        "GB".to_string(),
                        "QA".to_string(),
                        "OM".to_string(),
                        "RO".to_string(),
                        "RU".to_string(),
                        "RW".to_string(),
                        "WS".to_string(),
                        "SG".to_string(),
                        "ST".to_string(),
                        "ZA".to_string(),
                        "KR".to_string(),
                        "LK".to_string(),
                        "SH".to_string(),
                        "SD".to_string(),
                        "SR".to_string(),
                        "SZ".to_string(),
                        "SE".to_string(),
                        "CH".to_string(),
                        "SY".to_string(),
                        "TW".to_string(),
                        "TJ".to_string(),
                        "TZ".to_string(),
                        "TH".to_string(),
                        "TT".to_string(),
                        "TN".to_string(),
                        "TR".to_string(),
                        "UG".to_string(),
                        "UA".to_string(),
                        "US".to_string(),
                        "UZ".to_string(),
                        "VU".to_string(),
                        "VE".to_string(),
                        "VN".to_string(),
                        "ZM".to_string(),
                        "ZW".to_string(),
                    ],
                },
                value: None,
            },
        ),
        (
            "billing.address.city".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.address.city".to_string(),
                display_name: "city".to_string(),
                field_type: enums::FieldType::UserAddressCity,
                value: None,
            },
        ),
        (
            "billing.address.zip".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.address.zip".to_string(),
                display_name: "zip".to_string(),
                field_type: enums::FieldType::UserAddressPincode,
                value: None,
            },
        ),
    ])
}
