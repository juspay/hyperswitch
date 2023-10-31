use std::collections::{HashMap, HashSet};

use api_models::{enums, payment_methods::RequiredFieldInfo};
#[cfg(feature = "kms")]
use external_services::kms::KmsValue;

use super::settings::{ConnectorFields, Password, PaymentMethodType, RequiredFieldFinal};

impl Default for super::settings::Server {
    fn default() -> Self {
        Self {
            port: 8080,
            workers: num_cpus::get_physical(),
            host: "localhost".into(),
            request_body_limit: 16 * 1024, // POST request body is limited to 16KiB
            base_url: "http://localhost:8080".into(),
            shutdown_timeout: 30,
        }
    }
}

impl Default for super::settings::Database {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: Password::default(),
            host: "localhost".into(),
            port: 5432,
            dbname: String::new(),
            pool_size: 5,
            connection_timeout: 10,
            queue_strategy: Default::default(),
        }
    }
}

impl Default for super::settings::Proxy {
    fn default() -> Self {
        Self {
            http_url: Default::default(),
            https_url: Default::default(),
            idle_pool_connection_timeout: Some(90),
        }
    }
}

impl Default for super::settings::Locker {
    fn default() -> Self {
        Self {
            host: "localhost".into(),
            mock_locker: true,
            basilisk_host: "localhost".into(),
            locker_signing_key_id: "1".into(),
        }
    }
}

impl Default for super::settings::SupportedConnectors {
    fn default() -> Self {
        Self {
            wallets: ["klarna", "braintree"].map(Into::into).into(),
            /* cards: [
                "adyen",
                "authorizedotnet",
                "braintree",
                "checkout",
                "cybersource",
                "fiserv",
                "rapyd",
                "stripe",
            ]
            .map(Into::into)
            .into(), */
        }
    }
}

impl Default for super::settings::Refund {
    fn default() -> Self {
        Self {
            max_attempts: 10,
            max_age: 365,
        }
    }
}

impl Default for super::settings::EphemeralConfig {
    fn default() -> Self {
        Self { validity: 1 }
    }
}

#[cfg(feature = "kv_store")]
impl Default for super::settings::DrainerSettings {
    fn default() -> Self {
        Self {
            stream_name: "DRAINER_STREAM".into(),
            num_partitions: 64,
            max_read_count: 100,
            shutdown_interval: 1000,
            loop_interval: 500,
        }
    }
}

#[cfg(feature = "kv_store")]
impl Default for super::settings::KvConfig {
    fn default() -> Self {
        Self { ttl: 900 }
    }
}

use super::settings::{
    Mandates, SupportedConnectorsForMandate, SupportedPaymentMethodTypesForMandate,
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
                                ]),
                            },
                        ),
                        (
                            enums::PaymentMethodType::ApplePay,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([
                                    enums::Connector::Stripe,
                                    enums::Connector::Adyen,
                                ]),
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
                                    enums::Connector::Multisafepay,
                                    enums::Connector::Nexinets,
                                    enums::Connector::Noon,
                                    enums::Connector::Payme,
                                    enums::Connector::Stripe,
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
                                    enums::Connector::Multisafepay,
                                    enums::Connector::Nexinets,
                                    enums::Connector::Noon,
                                    enums::Connector::Payme,
                                    enums::Connector::Stripe,
                                ]),
                            },
                        ),
                    ])),
                ),
            ])),
        }
    }
}

impl Default for super::settings::RequiredFields {
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
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
                                            )
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressline1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.country".to_string(),
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.country".to_string(),
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            )
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressline1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line2".to_string(),
                                                    display_name: "line2".to_string(),
                                                    field_type: enums::FieldType::UserAddressline2,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.country".to_string(),
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
                                            )
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
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
                            ),(
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
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
                                                required_field: "billing.address.country".to_string(),
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
                                        )
                                    ]),
                                    common: HashMap::new(),
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
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
                                            )
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            )
                                        ]
                                    ),
                                    common: HashMap::new(),
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressline1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.state".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.state".to_string(),
                                                    display_name: "state".to_string(),
                                                    field_type: enums::FieldType::UserAddressState,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.country".to_string(),
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.country".to_string(),
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            )
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressline1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line2".to_string(),
                                                    display_name: "line2".to_string(),
                                                    field_type: enums::FieldType::UserAddressline2,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.city".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.city".to_string(),
                                                    display_name: "city".to_string(),
                                                    field_type: enums::FieldType::UserAddressCity,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.country".to_string(),
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
                                            )
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
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
                            ),(
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
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
                                                "payment_method_data.card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
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
                                                required_field: "billing.address.country".to_string(),
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
                                        )
                                    ]),
                                    common: HashMap::new(),
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
                        enums::PaymentMethodType::Przelewy24,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                enums::Connector::Stripe,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
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
                        enums::PaymentMethodType::Giropay,
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
                        enums::PaymentMethodType::Ideal,
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
                        enums::PaymentMethodType::Sofort,
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
                        enums::PaymentMethodType::Eps,
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
                                                "billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.last_name".to_string(),
                                                    display_name: "billing_last_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
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
                                )
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::GooglePay,
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
                            ]),
                        },
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
                                            ( "name".to_string(),
                                                RequiredFieldInfo {
                                                required_field: "name".to_string(),
                                                display_name: "cust_name".to_string(),
                                                field_type: enums::FieldType::UserFullName,
                                                value: None,
                                            }),
                                            ("payment_method_data.pay_later.afterpay_clearpay_redirect.billing_email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.pay_later.afterpay_clearpay_redirect.billing_email".to_string(),
                                                display_name: "billing_email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                           })
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
                                            ( "payment_method_data.pay_later.klarna.billing_country".to_string(),
                                                RequiredFieldInfo {
                                                required_field: "payment_method_data.pay_later.klarna.billing_country".to_string(),
                                                display_name: "billing_country".to_string(),
                                                field_type: enums::FieldType::UserAddressCountry{
                                                    options: vec![
                                                        "ALL".to_string(),
                                                    ]
                                                },
                                                value: None,
                                            }),
                                            ("email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "email".to_string(),
                                                display_name: "email".to_string(),
                                                field_type: enums::FieldType::UserEmailAddress,
                                                value: None,
                                           })
                                        ]),
                                        common : HashMap::new(),
                                    }
                                ),
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
                                                            "BUSD".to_string(),
                                                            "USDT".to_string(),
                                                            "USDC".to_string(),
                                                            "DAI".to_string(),
                                                        ]
                                                    },
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
                                        common: HashMap::new(),
                                    })]
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
                                        common: HashMap::new(),
                                    }
                                ),
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
                                        common: HashMap::new(),
                                    }
                                ),
                                ]),
                        },
                    )]))),
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
                                            non_mandate: HashMap::new(),
                                            common: HashMap::new(),
                                        }
                                    ),
                                ])}),
                                (enums::PaymentMethodType::Multibanco,
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
                                ])}),
                                (enums::PaymentMethodType::Ach,
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
                                ])}),
                                ])))
        ]))
    }
}

#[allow(clippy::derivable_impls)]
impl Default for super::settings::ApiKeys {
    fn default() -> Self {
        Self {
            #[cfg(feature = "kms")]
            kms_encrypted_hash_key: KmsValue::default(),

            /// Hex-encoded 32-byte long (64 characters long when hex-encoded) key used for calculating
            /// hashes of API keys
            #[cfg(not(feature = "kms"))]
            hash_key: String::new(),

            // Specifies the number of days before API key expiry when email reminders should be sent
            #[cfg(feature = "email")]
            expiry_reminder_days: vec![7, 3, 1],
        }
    }
}
