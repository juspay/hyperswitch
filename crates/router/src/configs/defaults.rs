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
                                enums::Connector::Stripe,
                                RequiredFieldFinal {
                                    mandate: HashMap::from([
                                        (
                                            "billing.address.line1".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "billing.address.line1".to_string(),
                                                display_name: "billing_line1".to_string(),
                                                field_type: enums::FieldType::UserFullName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "billing.address.first_name".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "billing.address.first_name".to_string(),
                                                display_name: "billing_first_name".to_string(),
                                                field_type: enums::FieldType::UserFullName,
                                                value: None,
                                            }
                                        )
                                    ]),
                                    non_mandate: HashMap::new(),
                                    common:HashMap::from([
                                        (
                                            "shipping.address.line1".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "shipping.address.line1".to_string(),
                                                display_name: "shipping_line1".to_string(),
                                                field_type: enums::FieldType::UserFullName,
                                                value: None,
                                            }
                                        ),
                                        (
                                            "shipping.phone.number".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "shipping.phone.number".to_string(),
                                                display_name: "shipping_phone".to_string(),
                                                field_type: enums::FieldType::UserFullName,
                                                value: None,
                                            }
                                        )
                                    ]),
                                }
                            ),
                            (
                            enums::Connector::Cybersource,
                            RequiredFieldFinal {
                                mandate: HashMap::from([
                                    (
                                        "billing.address.line1".to_string(),
                                        RequiredFieldInfo {
                                            required_field: "billing.address.line1".to_string(),
                                            display_name: "billing_line1".to_string(),
                                            field_type: enums::FieldType::UserFullName,
                                            value: None,
                                        }
                                    ),
                                    (
                                        "billing.address.first_name".to_string(),
                                        RequiredFieldInfo {
                                            required_field: "billing.address.first_name".to_string(),
                                            display_name: "billing_first_name".to_string(),
                                            field_type: enums::FieldType::UserFullName,
                                            value: None,
                                        }
                                    )
                                ]),
                                non_mandate: HashMap::new(),
                                common:HashMap::from([
                                    (
                                        "shipping.address.line1".to_string(),
                                        RequiredFieldInfo {
                                            required_field: "shipping.address.line1".to_string(),
                                            display_name: "shipping_line1".to_string(),
                                            field_type: enums::FieldType::UserFullName,
                                            value: None,
                                        }
                                    ),
                                    (
                                        "shipping.phone.number".to_string(),
                                        RequiredFieldInfo {
                                            required_field: "shipping.phone.number".to_string(),
                                            display_name: "shipping_phone".to_string(),
                                            field_type: enums::FieldType::UserFullName,
                                            value: None,
                                        }
                                    )
                                ]),
                            }
                            ),
                            (
                                enums::Connector::Dlocal,
                               RequiredFieldFinal {
                                mandate: HashMap::from([
                                    (
                                        "billing.address.line1".to_string(),
                                        RequiredFieldInfo {
                                            required_field: "billing.address.line1".to_string(),
                                            display_name: "billing_line1".to_string(),
                                            field_type: enums::FieldType::UserFullName,
                                            value: None,
                                        }
                                    ),
                                    (
                                        "billing.address.first_name".to_string(),
                                        RequiredFieldInfo {
                                            required_field: "billing.address.first_name".to_string(),
                                            display_name: "billing_first_name".to_string(),
                                            field_type: enums::FieldType::UserFullName,
                                            value: None,
                                        }
                                    )
                                ]),
                                non_mandate: HashMap::new(),
                                common:HashMap::from([
                                    (
                                        "shipping.address.line1".to_string(),
                                        RequiredFieldInfo {
                                            required_field: "shipping.address.line1".to_string(),
                                            display_name: "shipping_line1".to_string(),
                                            field_type: enums::FieldType::UserFullName,
                                            value: None,
                                        }
                                    ),
                                    (
                                        "shipping.phone.number".to_string(),
                                        RequiredFieldInfo {
                                            required_field: "shipping.phone.number".to_string(),
                                            display_name: "shipping_phone".to_string(),
                                            field_type: enums::FieldType::UserFullName,
                                            value: None,
                                        }
                                    )
                                ]),
                            }
                            ),
                            (
                                enums::Connector::Forte,
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
                                enums::Connector::Iatapay,
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
                                enums::Connector::Opennode,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
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
                                    non_mandate: HashMap::new(),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Worldline,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::new(),
                                }
                            ),
                            (
                                enums::Connector::Zen,
                                RequiredFieldFinal {
                                    mandate: HashMap::new(),
                                    non_mandate: HashMap::new(),
                                    common: HashMap::new(),
                                }
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
                                                display_name: "billing_name".to_string(),
                                                field_type: enums::FieldType::UserAddressCountry,
                                                value: None,
                                            }),
                                            ("email".to_string(),
                                            RequiredFieldInfo {
                                                required_field: "email".to_string(),
                                                display_name: "cust_email".to_string(),
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
