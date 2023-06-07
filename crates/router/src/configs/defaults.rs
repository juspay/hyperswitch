use std::collections::HashMap;

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
            locker_setup: super::settings::LockerSetup::LegacyLocker,
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

// Test Data
impl Default for super::settings::RequiredFields {
    fn default() -> Self {
        Self(HashMap::from([
            (
                enums::PaymentMethod::Card,
                PaymentMethodType(HashMap::from([(
                    enums::PaymentMethodType::Debit,
                    ConnectorFields {
                        fields: HashMap::from([(
                            enums::Connector::Stripe,
                            vec![RequiredFieldInfo {
                                required_field: Some("card_exp_year".to_string()),
                                display_name: Some("card_exp_year".to_string()),
                                field_type: Some("text".to_string()),
                                field_options: None,
                            }],
                        )]),
                    },
                )])),
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
                                            required_field: Some(
                                                "shipping.address.first_name".to_string(),
                                            ),
                                            display_name: Some("first_name".to_string()),
                                            field_type: Some("text".to_string()),
                                            field_options: None,
                                        },
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "shipping.address.line1".to_string(),
                                            ),
                                            display_name: Some("line1".to_string()),
                                            field_type: Some("text".to_string()),
                                            field_options: None,
                                        },
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "shipping.address.country".to_string(),
                                            ),
                                            display_name: Some("country".to_string()),
                                            field_type: Some("dropdown".to_string()),
                                            field_options: None,
                                        },
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "shipping.address.zip".to_string(),
                                            ),
                                            display_name: Some("zip".to_string()),
                                            field_type: Some("text".to_string()),
                                            field_options: None,
                                        },
                                    ],
                                ),
                                (
                                    enums::Connector::Adyen,
                                    vec![
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "shipping.address.first_name".to_string(),
                                            ),
                                            display_name: Some("first_name".to_string()),
                                            field_type: Some("text".to_string()),
                                            field_options: None,
                                        },
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "shipping.address.line1".to_string(),
                                            ),
                                            display_name: Some("line1".to_string()),
                                            field_type: Some("text".to_string()),
                                            field_options: None,
                                        },
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "shipping.address.country".to_string(),
                                            ),
                                            display_name: Some("country".to_string()),
                                            field_type: Some("dropdown".to_string()),
                                            field_options: None,
                                        },
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "shipping.address.zip".to_string(),
                                            ),
                                            display_name: Some("zip".to_string()),
                                            field_type: Some("text".to_string()),
                                            field_options: None,
                                        },
                                    ],
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::ApplePay,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Bluesnap,
                                vec![RequiredFieldInfo {
                                    required_field: Some("billing_address".to_string()),
                                    display_name: Some("billing_address".to_string()),
                                    field_type: Some("text".to_string()),
                                    field_options: None,
                                }],
                            )]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Ach,
                        ConnectorFields {
                            fields: HashMap::from([
                                (
                                    enums::Connector::Stripe,
                                    vec![RequiredFieldInfo {
                                        required_field: Some("currency".to_string()),
                                        display_name: Some("currency".to_string()),
                                        field_type: Some("text".to_string()),
                                        field_options: None,
                                    }],
                                ),
                                (
                                    enums::Connector::Adyen,
                                    vec![RequiredFieldInfo {
                                        required_field: Some("card_holder_name".to_string()),
                                        display_name: Some("card_holder_name".to_string()),
                                        field_type: Some("text".to_string()),
                                        field_options: None,
                                    }],
                                ),
                            ]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Przelewy24,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Stripe,
                                vec![RequiredFieldInfo {
                                    required_field: Some("bank_name".to_string()),
                                    display_name: Some("bank_name".to_string()),
                                    field_type: Some("text".to_string()),
                                    field_options: None,
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
                                        required_field: Some(
                                            "bancontact_card.billing_name".to_string(),
                                        ),
                                        display_name: Some("billing_name".to_string()),
                                        field_type: Some("text".to_string()),
                                        field_options: None,
                                    }],
                                ),
                                (
                                    enums::Connector::Adyen,
                                    vec![
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "bancontact_card.card_number".to_string(),
                                            ),
                                            display_name: Some("card_number".to_string()),
                                            field_type: Some("text".to_string()),
                                            field_options: None,
                                        },
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "bancontact_card.card_exp_month".to_string(),
                                            ),
                                            display_name: Some("card_exp_month".to_string()),
                                            field_type: Some("text".to_string()),
                                            field_options: None,
                                        },
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "bancontact_card.card_exp_year".to_string(),
                                            ),
                                            display_name: Some("card_exp_year".to_string()),
                                            field_type: Some("text".to_string()),
                                            field_options: None,
                                        },
                                        RequiredFieldInfo {
                                            required_field: Some(
                                                "bancontact_card.card_holder_name".to_string(),
                                            ),
                                            display_name: Some("card_holder_name".to_string()),
                                            field_type: Some("text".to_string()),
                                            field_options: None,
                                        },
                                    ],
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
                                    required_field: Some("bank_account_holder_name".to_string()),
                                    display_name: Some("bank_account_holder_name".to_string()),
                                    field_type: Some("text".to_string()),
                                    field_options: None,
                                }],
                            )]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Bacs,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Adyen,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: Some(
                                            "bancontact_card.billing_name".to_string(),
                                        ),
                                        display_name: Some("billing_name".to_string()),
                                        field_type: Some("text".to_string()),
                                        field_options: None,
                                    },
                                    RequiredFieldInfo {
                                        required_field: Some(
                                            "bank_account_holder_name".to_string(),
                                        ),
                                        display_name: Some("bank_account_holder_name".to_string()),
                                        field_type: Some("text".to_string()),
                                        field_options: None,
                                    },
                                ],
                            )]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Paypal,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Mollie,
                                vec![
                                    RequiredFieldInfo {
                                        required_field: Some("billing_address".to_string()),
                                        display_name: Some("billing_address".to_string()),
                                        field_type: Some("text".to_string()),
                                        field_options: None,
                                    },
                                    RequiredFieldInfo {
                                        required_field: Some("shipping_address".to_string()),
                                        display_name: Some("shipping_address".to_string()),
                                        field_type: Some("text".to_string()),
                                        field_options: None,
                                    },
                                ],
                            )]),
                        },
                    ),
                    (
                        enums::PaymentMethodType::Giropay,
                        ConnectorFields {
                            fields: HashMap::from([(
                                enums::Connector::Worldline,
                                vec![RequiredFieldInfo {
                                    required_field: Some(
                                        "billing_details.billing_name".to_string(),
                                    ),
                                    display_name: Some("billing_name".to_string()),
                                    field_type: Some("text".to_string()),
                                    field_options: None,
                                }],
                            )]),
                        },
                    ),
                ])),
            ),
        ]))
    }
}
