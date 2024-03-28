use std::collections::{HashMap, HashSet};

use api_models::{enums, payment_methods::RequiredFieldInfo};

use super::settings::{ConnectorFields, PaymentMethodType, RequiredFieldFinal};

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

impl Default for super::settings::CorsSettings {
    fn default() -> Self {
        Self {
            origins: HashSet::from_iter(["http://localhost:8080".to_string()]),
            allowed_methods: HashSet::from_iter(
                ["GET", "PUT", "POST", "DELETE"]
                    .into_iter()
                    .map(ToString::to_string),
            ),
            wildcard_origin: false,
            max_age: 30,
        }
    }
}
impl Default for super::settings::Database {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: String::new().into(),
            host: "localhost".into(),
            port: 5432,
            dbname: String::new(),
            pool_size: 5,
            connection_timeout: 10,
            queue_strategy: Default::default(),
            min_idle: None,
            max_lifetime: None,
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
            host_rs: "localhost".into(),
            mock_locker: true,
            basilisk_host: "localhost".into(),
            locker_signing_key_id: "1".into(),
            //true or false
            locker_enabled: true,
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
            loop_interval: 100,
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
                                    enums::Connector::Airwallex,
                                    enums::Connector::Authorizedotnet,
                                    enums::Connector::Bankofamerica,
                                    enums::Connector::Bluesnap,
                                    enums::Connector::Checkout,
                                    enums::Connector::Globalpay,
                                    enums::Connector::Multisafepay,
                                    enums::Connector::Noon,
                                    enums::Connector::Nuvei,
                                    enums::Connector::Payu,
                                    enums::Connector::Rapyd,
                                    enums::Connector::Stripe,
                                    enums::Connector::Trustpay,
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
            update_mandate_supported: SupportedPaymentMethodsForMandate(HashMap::default()),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                    field_type: enums::FieldType::UserAddressLine1,
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
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
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line2".to_string(),
                                                    display_name: "line2".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine2,
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
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.zip".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                    required_field: "billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                    field_type: enums::FieldType::UserAddressLine1,
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.last_name".to_string(),
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
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.zip".to_string(),
                                                    display_name: "zip".to_string(),
                                                    field_type: enums::FieldType::UserAddressPincode,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                    field_type: enums::FieldType::UserAddressLine1,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line2".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line2".to_string(),
                                                    display_name: "line2".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine2,
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
                                                "billing.address.zip".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.zip".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                "payment_method_data.billing.address.first_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.billing.address.first_name".to_string(),
                                                    display_name: "card_holder_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.billing.address.last_name".to_string(),
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
                                                    required_field: "billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
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
                                        mandate: HashMap::new(),
                                        non_mandate: HashMap::new(),
                                        common: HashMap::from([
                                            (
                                                "payment_method_data.bank_redirect.bancontact_card.billing_details.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.bancontact_card.billing_details.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_redirect.bancontact_card.billing_details.billing_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.bancontact_card.billing_details.billing_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
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
                                                "payment_method_data.bank_redirect.bancontact_card.card_holder_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.bancontact_card.card_holder_name".to_string(),
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
                                                "payment_method_data.bank_redirect.giropay.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.giropay.country".to_string(),
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
                                                required_field: "billing.address.country".to_string(),
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
                                            ("payment_method_data.bank_redirect.giropay.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.giropay.country".to_string(),
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
                                            "payment_method_data.bank_redirect.giropay.billing_details.billing_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.giropay.billing_details.billing_name".to_string(),
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
                                            ("payment_method_data.bank_redirect.giropay.billing_details.billing_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.giropay.billing_details.billing_name".to_string(),
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
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
                                                "payment_method_data.bank_redirect.ideal.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.ideal.country".to_string(),
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
                                                "payment_method_data.bank_redirect.ideal.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.ideal.country".to_string(),
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
                                                "payment_method_data.bank_redirect.ideal.billing_details.billing_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.ideal.billing_details.billing_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_redirect.ideal.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.ideal.country".to_string(),
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
                                                "payment_method_data.bank_redirect.ideal.billing_details.billing_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.ideal.billing_details.billing_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_redirect.ideal.billing_details.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.ideal.billing_details.email".to_string(),
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
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
                                            ("payment_method_data.bank_redirect.sofort.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.sofort.country".to_string(),
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
                                                required_field: "billing.address.country".to_string(),
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
                                            ("payment_method_data.bank_redirect.sofort.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.sofort.country".to_string(),
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
                                            "payment_method_data.bank_redirect.sofort.billing_details.billing_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.sofort.billing_details.billing_name".to_string(),
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
                                                "payment_method_data.bank_redirect.sofort.billing_details.email".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.sofort.billing_details.email".to_string(),
                                                    display_name: "email".to_string(),
                                                    field_type: enums::FieldType::UserEmailAddress,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_redirect.sofort.billing_details.billing_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.sofort.billing_details.billing_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            )
                                        ]),
                                        non_mandate : HashMap::from([
                                            ("payment_method_data.bank_redirect.sofort.country".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "payment_method_data.bank_redirect.sofort.country".to_string(),
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
                                        )]),
                                        common: HashMap::new(

                                        ),
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
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
                                                "payment_method_data.bank_redirect.eps.billing_details.billing_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.eps.billing_details.billing_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            )
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
                                                "payment_method_data.bank_redirect.eps.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.eps.country".to_string(),
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
                                                    required_field: "billing.address.country".to_string(),
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
                                                "payment_method_data.bank_redirect.eps.billing_details.billing_name".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.eps.billing_details.billing_name".to_string(),
                                                    display_name: "billing_name".to_string(),
                                                    field_type: enums::FieldType::UserFullName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "payment_method_data.bank_redirect.eps.country".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "payment_method_data.bank_redirect.eps.country".to_string(),
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
                                                    required_field: "billing.address.first_name".to_string(),
                                                    display_name: "billing_first_name".to_string(),
                                                    field_type: enums::FieldType::UserBillingName,
                                                    value: None,
                                                }
                                            ),
                                            (
                                                "billing.address.line1".to_string(),
                                                RequiredFieldInfo {
                                                    required_field: "billing.address.line1".to_string(),
                                                    display_name: "line1".to_string(),
                                                    field_type: enums::FieldType::UserAddressLine1,
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
                                                    field_type: enums::FieldType::UserAddressLine1,
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
                                    enums::Connector::Bankofamerica,
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
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "billing.address.line1".to_string(),
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
                                    enums::Connector::Cybersource,
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
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "billing.address.line1".to_string(),
                                                        display_name: "line1".to_string(),
                                                        field_type: enums::FieldType::UserAddressLine1,
                                                        value: None,
                                                    }
                                                ),
                                            ]
                                        ),
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
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "billing.address.line1".to_string(),
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
                                        ),
                                        (
                                            "billing.address.line1".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "billing.address.line1".to_string(),
                                                display_name: "line1".to_string(),
                                                field_type: enums::FieldType::UserAddressLine1,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.line2".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "billing.address.line2".to_string(),
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
                                                ),
                                                (
                                                    "billing.address.line1".to_string(),
                                                    RequiredFieldInfo {
                                                        required_field: "billing.address.line1".to_string(),
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
                                        common: HashMap::from([ (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "billing.address.first_name".to_string(),
                                                display_name: "billing_first_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        )]),
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
                                        common: HashMap::from([ (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "billing.address.first_name".to_string(),
                                                display_name: "billing_first_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        )]),
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
                                        common: HashMap::from([ (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "billing.address.first_name".to_string(),
                                                display_name: "billing_first_name".to_string(),
                                                field_type: enums::FieldType::UserBillingName,
                                                value: None,
                                            }
                                        )]),
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
            // Hex-encoded 32-byte long (64 characters long when hex-encoded) key used for calculating
            // hashes of API keys
            hash_key: String::new().into(),

            // Specifies the number of days before API key expiry when email reminders should be sent
            #[cfg(feature = "email")]
            expiry_reminder_days: vec![7, 3, 1],
        }
    }
}
