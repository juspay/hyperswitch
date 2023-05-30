use std::collections::HashMap;

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
            migration_encryption_timestamp: 0,
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
                api_models::enums::PaymentMethod::PayLater,
                HashMap::from([
                    (
                        api_models::enums::PaymentMethodType::AfterpayClearpay,
                        HashMap::from([(
                            "stripe".to_string(),
                            vec![
                                "shipping.address.first_name".to_string(),
                                "shipping.address.line1".to_string(),
                                "shipping.address.country".to_string(),
                                "shipping.address.zip".to_string(),
                            ],
                        )]),
                    ),
                    (
                        api_models::enums::PaymentMethodType::ApplePay,
                        HashMap::from([(
                            "bluesnap".to_string(),
                            vec!["billing_address".to_string()],
                        )]),
                    ),
                    (
                        api_models::enums::PaymentMethodType::Ach,
                        HashMap::from([
                            ("stripe".to_string(), vec!["currency".to_string()]),
                            ("adyen".to_string(), vec!["card_holder_name".to_string()]),
                        ]),
                    ),
                    (
                        api_models::enums::PaymentMethodType::Przelewy24,
                        HashMap::from([("stripe".to_string(), vec!["bank_name".to_string()])]),
                    ),
                    (
                        api_models::enums::PaymentMethodType::BancontactCard,
                        HashMap::from([
                            (
                                "stripe".to_string(),
                                vec!["bancontact_card.billing_name".to_string()],
                            ),
                            (
                                "adyen".to_string(),
                                vec![
                                    "bancontact_card.card_number".to_string(),
                                    "bancontact_card.card_exp_month".to_string(),
                                    "bancontact_card.card_exp_year".to_string(),
                                    "bancontact_card.card_holder_name".to_string(),
                                ],
                            ),
                        ]),
                    ),
                    (
                        api_models::enums::PaymentMethodType::Sepa,
                        HashMap::from([(
                            "adyen".to_string(),
                            vec!["bank_account_holder_name".to_string()],
                        )]),
                    ),
                    (
                        api_models::enums::PaymentMethodType::Bacs,
                        HashMap::from([(
                            "adyen".to_string(),
                            vec![
                                "bancontact_card.billing_name".to_string(),
                                "bank_account_holder_name".to_string(),
                            ],
                        )]),
                    ),
                    (
                        api_models::enums::PaymentMethodType::Paypal,
                        HashMap::from([(
                            "mollie".to_string(),
                            vec!["billing_address".to_string(), "shippy_address".to_string()],
                        )]),
                    ),
                    (
                        api_models::enums::PaymentMethodType::Giropay,
                        HashMap::from([(
                            "wordline".to_string(),
                            vec!["billing_details.billing_name".to_string()],
                        )]),
                    ),
                ]),
            ),
            (
                api_models::enums::PaymentMethod::Card,
                HashMap::from([(
                    api_models::enums::PaymentMethodType::Debit,
                    HashMap::from([("stripe".to_string(), vec!["card_exp_year".to_string()])]),
                )]),
            ),
        ]))
    }
}
