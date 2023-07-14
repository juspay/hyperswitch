use std::collections::{HashMap, HashSet};

use api_models::{enums, payment_methods::RequiredFieldInfo};

use super::settings::{ConnectorFields, PaymentMethodType};

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
            #[cfg(not(feature = "kms"))]
            password: String::new(),
            host: "localhost".into(),
            port: 5432,
            dbname: String::new(),
            pool_size: 5,
            connection_timeout: 10,
            #[cfg(feature = "kms")]
            kms_encrypted_password: String::new(),
        }
    }
}

impl Default for super::settings::Secrets {
    fn default() -> Self {
        Self {
            #[cfg(not(feature = "kms"))]
            jwt_secret: "secret".into(),
            #[cfg(not(feature = "kms"))]
            admin_api_key: "test_admin".into(),
            master_enc_key: "".into(),
            #[cfg(feature = "kms")]
            kms_encrypted_jwt_secret: "".into(),
            #[cfg(feature = "kms")]
            kms_encrypted_admin_api_key: "".into(),
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

impl Default for super::settings::SchedulerSettings {
    fn default() -> Self {
        Self {
            stream: "SCHEDULER_STREAM".into(),
            producer: super::settings::ProducerSettings::default(),
            consumer: super::settings::ConsumerSettings::default(),
            graceful_shutdown_interval: 60000,
            loop_interval: 5000,
        }
    }
}

impl Default for super::settings::ProducerSettings {
    fn default() -> Self {
        Self {
            upper_fetch_limit: 0,
            lower_fetch_limit: 1800,
            lock_key: "PRODUCER_LOCKING_KEY".into(),
            lock_ttl: 160,
            batch_size: 200,
        }
    }
}

impl Default for super::settings::ConsumerSettings {
    fn default() -> Self {
        Self {
            disabled: false,
            consumer_group: "SCHEDULER_GROUP".into(),
        }
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
                                    enums::Connector::Stripe,
                                    enums::Connector::Adyen,
                                    enums::Connector::Authorizedotnet,
                                    enums::Connector::Globalpay,
                                    enums::Connector::Worldpay,
                                    enums::Connector::Multisafepay,
                                    enums::Connector::Nmi,
                                    enums::Connector::Nexinets,
                                    enums::Connector::Noon,
                                ]),
                            },
                        ),
                        (
                            enums::PaymentMethodType::Debit,
                            SupportedConnectorsForMandate {
                                connector_list: HashSet::from([
                                    enums::Connector::Stripe,
                                    enums::Connector::Adyen,
                                    enums::Connector::Authorizedotnet,
                                    enums::Connector::Globalpay,
                                    enums::Connector::Worldpay,
                                    enums::Connector::Multisafepay,
                                    enums::Connector::Nmi,
                                    enums::Connector::Nexinets,
                                    enums::Connector::Noon,
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
                PaymentMethodType(HashMap::from([(
                    enums::PaymentMethodType::Debit,
                    ConnectorFields {
                        fields: HashMap::from([
                            (
                                enums::Connector::Aci,
                                vec![RequiredFieldInfo {
                                    required_field: "payment_method_data.card.card_holder_name".to_string(),
                                    display_name: "card_holder_name".to_string(),
                                    field_type: enums::FieldType::UserFullName,
                                }],
                            ),
                            (
                                enums::Connector::Bluesnap,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: "email".to_string(),
                                        display_name: "email".to_string(),
                                        field_type: enums::FieldType::UserEmailAddress,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.first_name".to_string(),
                                        display_name: "first_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.last_name".to_string(),
                                        display_name: "last_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                ],
                            ),
                            (
                                enums::Connector::Bambora,
                                vec![RequiredFieldInfo {
                                    required_field: "card.card_holder_name".to_string(),
                                    display_name: "card_holder_name".to_string(),
                                    field_type: enums::FieldType::UserFullName,
                                }],
                            ),
                            (
                                enums::Connector::Cybersource,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: "email".to_string(),
                                        display_name: "email".to_string(),
                                        field_type: enums::FieldType::UserEmailAddress,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.phone.number".to_string(),
                                        display_name: "phone_number".to_string(),
                                        field_type: enums::FieldType::UserPhoneNumber,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.phone.country_code".to_string(),
                                        display_name: "country".to_string(),
                                        field_type: enums::FieldType::UserCountry {
                                            options: vec!["US".to_string(), "IN".to_string()],
                                        },
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.first_name".to_string(),
                                        display_name: "first_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.last_name".to_string(),
                                        display_name: "last_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.line1".to_string(),
                                        display_name: "line1".to_string(),
                                        field_type: enums::FieldType::UserAddressline1,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.city".to_string(),
                                        display_name: "city".to_string(),
                                        field_type: enums::FieldType::UserAddressCity,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.state".to_string(),
                                        display_name: "state".to_string(),
                                        field_type: enums::FieldType::UserAddressState,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.zip".to_string(),
                                        display_name: "zip".to_string(),
                                        field_type: enums::FieldType::UserAddressPincode,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.country".to_string(),
                                        display_name: "country".to_string(),
                                        field_type: enums::FieldType::UserCountry {
                                            options: vec!["US".to_string(), "IN".to_string()],
                                        },
                                    },
                                ],
                            ),
                            (
                                enums::Connector::Dlocal,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: "card.card_holder_name".to_string(),
                                        display_name: "card_holder_name".to_string(),
                                        field_type: enums::FieldType::UserFullName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "email".to_string(),
                                        display_name: "email".to_string(),
                                        field_type: enums::FieldType::UserEmailAddress,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.country".to_string(),
                                        display_name: "country".to_string(),
                                        field_type: enums::FieldType::UserCountry {
                                            options: vec!["US".to_string(), "IN".to_string()],
                                        },
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.first_name".to_string(),
                                        display_name: "first_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.last_name".to_string(),
                                        display_name: "last_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                ],
                            ),
                            (
                                enums::Connector::Forte,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: "card.card_holder_name".to_string(),
                                        display_name: "card_holder_name".to_string(),
                                        field_type: enums::FieldType::UserFullName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.first_name".to_string(),
                                        display_name: "first_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.last_name".to_string(),
                                        display_name: "last_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                ],
                            ),
                            (
                                enums::Connector::Globalpay,
                                vec![RequiredFieldInfo {
                                    required_field: "billing.address.country".to_string(),
                                    display_name: "country".to_string(),
                                    field_type: enums::FieldType::UserCountry {
                                        options: vec!["US".to_string(), "IN".to_string()],
                                    },
                                }],
                            ),
                            (
                                enums::Connector::Iatapay,
                                vec![RequiredFieldInfo {
                                    required_field: "billing.address.country".to_string(),
                                    display_name: "country".to_string(),
                                    field_type: enums::FieldType::UserCountry {
                                        options: vec!["US".to_string(), "IN".to_string()],
                                    },
                                }],
                            ),
                            (
                                enums::Connector::Multisafepay,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: "billing.address.first_name".to_string(),
                                        display_name: "first_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.last_name".to_string(),
                                        display_name: "last_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.line1".to_string(),
                                        display_name: "line1".to_string(),
                                        field_type: enums::FieldType::UserAddressline1,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.line2".to_string(),
                                        display_name: "line2".to_string(),
                                        field_type: enums::FieldType::UserAddressline2,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.city".to_string(),
                                        display_name: "city".to_string(),
                                        field_type: enums::FieldType::UserAddressCity,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.zip".to_string(),
                                        display_name: "zip".to_string(),
                                        field_type: enums::FieldType::UserAddressPincode,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.country".to_string(),
                                        display_name: "country".to_string(),
                                        field_type: enums::FieldType::UserCountry{
                                            options: vec!["US".to_string(), "IN".to_string()],
                                        },
                                    },
                                ],
                            ),
                            (
                                enums::Connector::Noon,
                                vec![RequiredFieldInfo {
                                    required_field: "card.card_holder_name".to_string(),
                                    display_name: "card_holder_name".to_string(),
                                    field_type: enums::FieldType::UserFullName,
                                }],
                            ),
                            (
                                enums::Connector::Opennode,
                                vec![RequiredFieldInfo {
                                    required_field: "description".to_string(),
                                    display_name: "description".to_string(),
                                    field_type: enums::FieldType::Text,
                                }],
                            ),
                            (
                                enums::Connector::Payu,
                                vec![RequiredFieldInfo {
                                    required_field: "description".to_string(),
                                    display_name: "description".to_string(),
                                    field_type: enums::FieldType::Text,
                                }],
                            ),
                            (
                                enums::Connector::Rapyd,
                                vec![RequiredFieldInfo {
                                    required_field: "card.card_holder_name".to_string(),
                                    display_name: "card_holder_name".to_string(),
                                    field_type: enums::FieldType::UserFullName,
                                }],
                            ),
                            (
                                enums::Connector::Shift4,
                                vec![RequiredFieldInfo {
                                    required_field: "card.card_holder_name".to_string(),
                                    display_name: "card_holder_name".to_string(),
                                    field_type: enums::FieldType::UserFullName,
                                }],
                            ),
                            (
                                enums::Connector::Trustpay,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: "card.card_holder_name".to_string(),
                                        display_name: "card_holder_name".to_string(),
                                        field_type: enums::FieldType::UserFullName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.line1".to_string(),
                                        display_name: "line1".to_string(),
                                        field_type: enums::FieldType::UserAddressline1,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.city".to_string(),
                                        display_name: "city".to_string(),
                                        field_type: enums::FieldType::UserAddressCity,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.zip".to_string(),
                                        display_name: "zip".to_string(),
                                        field_type: enums::FieldType::UserAddressPincode,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.country".to_string(),
                                        display_name: "country".to_string(),
                                        field_type: enums::FieldType::UserCountry {
                                            options: vec!["US".to_string(), "IN".to_string()],
                                        },
                                    },
                                    RequiredFieldInfo {
                                        required_field: "email".to_string(),
                                        display_name: "email".to_string(),
                                        field_type: enums::FieldType::UserEmailAddress,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "browser_info".to_string(),
                                        display_name: "browser_info".to_string(),
                                        field_type: enums::FieldType::Text,
                                    },
                                ],
                            ),
                            (
                                enums::Connector::Worldline,
                                vec![RequiredFieldInfo {
                                    required_field: "card.card_holder_name".to_string(),
                                    display_name: "card_holder_name".to_string(),
                                    field_type: enums::FieldType::UserFullName,
                                }],
                            ),
                            (
                                enums::Connector::Zen,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: "email".to_string(),
                                        display_name: "email".to_string(),
                                        field_type: enums::FieldType::UserEmailAddress,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "browser_info".to_string(),
                                        display_name: "browser_info".to_string(),
                                        field_type: enums::FieldType::Text,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "description".to_string(),
                                        display_name: "description".to_string(),
                                        field_type: enums::FieldType::Text,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "metadata.order_details".to_string(),
                                        display_name: "order_details".to_string(),
                                        field_type: enums::FieldType::Text,
                                    },
                                ],
                            ),
                        ]),
                    },
                )])),
            ),
            (
                enums::PaymentMethod::BankRedirect,
                PaymentMethodType(HashMap::from([
                    (
                        enums::PaymentMethodType::Przelewy24,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Stripe,
                                vec![RequiredFieldInfo {
                                    required_field: "payment_method_data.bank_redirect.przelewy24.bank_name".to_string(),
                                    display_name: "bank_name".to_string(),
                                    field_type: enums::FieldType::UserBank,
                                }],
                            )]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::BancontactCard,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    vec![RequiredFieldInfo {
                                        required_field: "payment_method_data.bank_redirect.bancontact_card.billing_details.billing_name".to_string(),
                                        display_name: "billing_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    }],
                                ),
                                (
                                    enums::Connector::Adyen,
                                    vec![
                                        RequiredFieldInfo {
                                            required_field: "payment_method_data.bank_redirect.bancontact_card.card_number"
                                                .to_string(),
                                            display_name: "card_number".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "payment_method_data.bank_redirect.bancontact_card.card_exp_month"
                                                .to_string(),
                                            display_name: "card_exp_month".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "payment_method_data.bank_redirect.bancontact_card.card_exp_year"
                                                .to_string(),
                                            display_name: "card_exp_year".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "payment_method_data.bank_redirect.bancontact_card.card_holder_name"
                                                .to_string(),
                                            display_name: "card_holder_name".to_string(),
                                            field_type: enums::FieldType::UserFullName,
                                        },
                                    ],
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Giropay,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Worldline,
                                    vec![RequiredFieldInfo {
                                        required_field: "payment_method_data.bank_redirect.giropay.billing_details.billing_name"
                                            .to_string(),
                                        display_name: "billing_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    }],
                                ),
                                (
                                    enums::Connector::Nuvei,
                                    vec![
                                        RequiredFieldInfo {
                                            required_field: "email".to_string(),
                                            display_name: "email".to_string(),
                                            field_type: enums::FieldType::UserEmailAddress,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "billing.address.country".to_string(),
                                            display_name: "country".to_string(),
                                            field_type: enums::FieldType::UserCountry {
                                                options: vec!["US".to_string(), "IN".to_string()],
                                            },
                                        },
                                    ],
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Ideal,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Worldline,
                                    vec![RequiredFieldInfo {
                                        required_field: "payment_method_data.bank_redirect.ideal.bank_name".to_string(),
                                        display_name: "bank_name".to_string(),
                                        field_type: enums::FieldType::UserBank,
                                    }],
                                ),
                                (
                                    enums::Connector::Nuvei,
                                    vec![
                                        RequiredFieldInfo {
                                            required_field: "payment_method_data.bank_redirect.ideal.bank_name".to_string(),
                                            display_name: "bank_name".to_string(),
                                            field_type: enums::FieldType::UserBank,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "billing.address.first_name"
                                                .to_string(),
                                            display_name: "first_name".to_string(),
                                            field_type: enums::FieldType::UserBillingName,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "billing.address.last_name".to_string(),
                                            display_name: "last_name".to_string(),
                                            field_type: enums::FieldType::UserBillingName,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "email".to_string(),
                                            display_name: "email".to_string(),
                                            field_type: enums::FieldType::UserEmailAddress,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "billing.address.country".to_string(),
                                            display_name: "country".to_string(),
                                            field_type: enums::FieldType::UserCountry {
                                                options: vec!["US".to_string(), "IN".to_string()],
                                            },
                                        },
                                    ],
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Sofort,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Nuvei,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: "billing.address.first_name".to_string(),
                                        display_name: "first_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.last_name".to_string(),
                                        display_name: "last_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "email".to_string(),
                                        display_name: "email".to_string(),
                                        field_type: enums::FieldType::UserEmailAddress,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.country".to_string(),
                                        display_name: "country".to_string(),
                                        field_type: enums::FieldType::UserCountry {
                                            options: vec!["US".to_string(), "IN".to_string()],
                                        },
                                    },
                                ],
                            )]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Eps,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Nuvei,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: "billing.address.first_name".to_string(),
                                        display_name: "first_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.last_name".to_string(),
                                        display_name: "last_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "email".to_string(),
                                        display_name: "email".to_string(),
                                        field_type: enums::FieldType::UserEmailAddress,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.country".to_string(),
                                        display_name: "country".to_string(),
                                        field_type: enums::FieldType::UserCountry {
                                            options: vec!["US".to_string(), "IN".to_string()],
                                        },
                                    },
                                ],
                            )]),
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
                                    enums::Connector::Bluesnap,
                                    vec![RequiredFieldInfo {
                                        required_field: "billing_address".to_string(),
                                        display_name: "billing_address".to_string(),
                                        field_type: enums::FieldType::Text,
                                    }],
                                ),
                                (
                                    enums::Connector::Zen,
                                    vec![RequiredFieldInfo {
                                        required_field: "metadata.order_details".to_string(),
                                        display_name: "order_details".to_string(),
                                        field_type: enums::FieldType::Text,
                                    }],
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Paypal,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Mollie,
                                    vec![
                                        RequiredFieldInfo {
                                            required_field: "billing_address".to_string(),
                                            display_name: "billing_address".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "shipping_address".to_string(),
                                            display_name: "shipping_address".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                    ],
                                ),
                                (
                                    enums::Connector::Nuvei,
                                    vec![
                                        RequiredFieldInfo {
                                            required_field: "email".to_string(),
                                            display_name: "email".to_string(),
                                            field_type: enums::FieldType::UserEmailAddress,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "billing.address.country".to_string(),
                                            display_name: "country".to_string(),
                                            field_type: enums::FieldType::UserCountry {
                                                options: vec!["US".to_string(), "IN".to_string()],
                                            },
                                        },
                                    ],
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::GooglePay,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Zen,
                                vec![RequiredFieldInfo {
                                    required_field: "metadata.order_details".to_string(),
                                    display_name: "order_details".to_string(),
                                    field_type: enums::FieldType::Text,
                                }],
                            )]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::AliPay,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Globepay,
                                vec![RequiredFieldInfo {
                                    required_field: "description".to_string(),
                                    display_name: "description".to_string(),
                                    field_type: enums::FieldType::Text,
                                }],
                            )]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::WeChatPay,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Globepay,
                                vec![RequiredFieldInfo {
                                    required_field: "description".to_string(),
                                    display_name: "description".to_string(),
                                    field_type: enums::FieldType::Text,
                                }],
                            )]),
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
                                    vec![
                                        RequiredFieldInfo {
                                            required_field: "shipping.address.first_name"
                                                .to_string(),
                                            display_name: "first_name".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "shipping.address.line1".to_string(),
                                            display_name: "line1".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "shipping.address.country".to_string(),
                                            display_name: "country".to_string(),
                                            field_type: enums::FieldType::DropDown {
                                                options: vec!["US".to_string(), "IN".to_string()],
                                            },
                                        },
                                        RequiredFieldInfo {
                                            required_field: "shipping.address.zip".to_string(),
                                            display_name: "zip".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                    ],
                                ),
                                (
                                    enums::Connector::Adyen,
                                    vec![
                                        RequiredFieldInfo {
                                            required_field: "shipping.address.first_name"
                                                .to_string(),
                                            display_name: "first_name".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "shipping.address.line1".to_string(),
                                            display_name: "line1".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "shipping.address.country".to_string(),
                                            display_name: "country".to_string(),
                                            field_type: enums::FieldType::DropDown {
                                                options: vec!["US".to_string(), "IN".to_string()],
                                            },
                                        },
                                        RequiredFieldInfo {
                                            required_field: "shipping.address.zip".to_string(),
                                            display_name: "zip".to_string(),
                                            field_type: enums::FieldType::Text,
                                        },
                                    ],
                                ),
                                (
                                    enums::Connector::Nuvei,
                                    vec![
                                        RequiredFieldInfo {
                                            required_field: "billing.address.first_name"
                                                .to_string(),
                                            display_name: "first_name".to_string(),
                                            field_type: enums::FieldType::UserBillingName,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "billing.address.last_name".to_string(),
                                            display_name: "last_name".to_string(),
                                            field_type: enums::FieldType::UserBillingName,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "email".to_string(),
                                            display_name: "email".to_string(),
                                            field_type: enums::FieldType::UserEmailAddress,
                                        },
                                        RequiredFieldInfo {
                                            required_field: "billing.address.country".to_string(),
                                            display_name: "country".to_string(),
                                            field_type: enums::FieldType::UserCountry {
                                                options: vec!["US".to_string(), "IN".to_string()],
                                            },
                                        },
                                    ],
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Klarna,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Nuvei,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: "billing.address.first_name".to_string(),
                                        display_name: "first_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.last_name".to_string(),
                                        display_name: "last_name".to_string(),
                                        field_type: enums::FieldType::UserBillingName,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "email".to_string(),
                                        display_name: "email".to_string(),
                                        field_type: enums::FieldType::UserEmailAddress,
                                    },
                                    RequiredFieldInfo {
                                        required_field: "billing.address.country".to_string(),
                                        display_name: "country".to_string(),
                                        field_type: enums::FieldType::UserCountry {
                                            options: vec!["US".to_string(), "IN".to_string()],
                                        },
                                    },
                                ],
                            )]),
                        },
                    ),
                ])),
            ),
            (
                enums::PaymentMethod::Crypto,
                PaymentMethodType(HashMap::from([(
                    enums::PaymentMethodType::CryptoCurrency,
                    ConnectorFields {
                        fields: HashMap::from([(
                            enums::Connector::Cryptopay,
                            vec![RequiredFieldInfo {
                                required_field: "payment_method_data.crypto.pay_currency".to_string(),
                                display_name: "currency".to_string(),
                                field_type: enums::FieldType::DropDown {
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
                                    ],
                                },
                            }],
                        )]),
                    },
                )])),
            ),
            (
                enums::PaymentMethod::BankDebit,
                PaymentMethodType(HashMap::from([(
                    enums::PaymentMethodType::Ach,
                    ConnectorFields {
                        fields: HashMap::from([
                            (
                                enums::Connector::Adyen,
                                vec![RequiredFieldInfo {
                                    required_field: "card_holder_name".to_string(),
                                    display_name: "card_holder_name".to_string(),
                                    field_type: enums::FieldType::UserFullName,
                                }],
                            ),
                        ]),
                    },
                ),
                (
                        enums::PaymentMethodType::Sepa,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Adyen,
                                vec![RequiredFieldInfo {
                                    required_field: "payment_method_data.bank_debit.sepa_bank_debit.bank_account_holder_name".to_string(),
                                    display_name: "bank_account_holder_name".to_string(),
                                    field_type: enums::FieldType::UserFullName,
                                }],
                            )]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Bacs,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Adyen,
                                vec![RequiredFieldInfo {
                                    required_field: "payment_method_data.bank_debit.bacs_bank_debit.bank_account_holder_name".to_string(),
                                    display_name: "bank_account_holder_name".to_string(),
                                    field_type: enums::FieldType::UserFullName,
                                }],
                            )]),
                        },
                    ),
                ])),
            ),
        ]))
    }
}

#[allow(clippy::derivable_impls)]
impl Default for super::settings::ApiKeys {
    fn default() -> Self {
        Self {
            #[cfg(feature = "kms")]
            kms_encrypted_hash_key: String::new(),

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
