use std::collections::{HashMap, HashSet};

use api_models::{enums, payment_methods::RequiredFieldInfo};
use common_utils::id_type;

#[cfg(feature = "payouts")]
pub mod payout_required_fields;

pub mod payment_connector_required_fields;

impl Default for super::settings::Server {
    fn default() -> Self {
        Self {
            port: 8080,
            workers: num_cpus::get_physical(),
            host: "localhost".into(),
            request_body_limit: 16 * 1024, // POST request body is limited to 16KiB
            shutdown_timeout: 30,
            #[cfg(feature = "tls")]
            tls: None,
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
            bypass_proxy_hosts: Default::default(),
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
            //Time to live for storage entries in locker
            ttl_for_storage_in_secs: 60 * 60 * 24 * 365 * 7,
            decryption_scheme: Default::default(),
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
        Self {
            ttl: 900,
            soft_kill: Some(false),
        }
    }
}

impl Default for super::settings::GlobalTenant {
    fn default() -> Self {
        Self {
            tenant_id: id_type::TenantId::get_default_global_tenant_id(),
            schema: String::from("global"),
            redis_key_prefix: String::from("global"),
            clickhouse_database: String::from("global"),
        }
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

            // Hex-encoded key used for calculating checksum for partial auth
            #[cfg(feature = "partial-auth")]
            checksum_auth_key: String::new().into(),
            // context used for blake3
            #[cfg(feature = "partial-auth")]
            checksum_auth_context: String::new().into(),

            #[cfg(feature = "partial-auth")]
            enable_partial_auth: false,
        }
    }
}

pub fn get_billing_required_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        (
            "billing.address.first_name".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.address.first_name".to_string(),
                display_name: "billing_first_name".to_string(),
                field_type: enums::FieldType::UserBillingName,
                value: None,
            },
        ),
        (
            "billing.address.last_name".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.address.last_name".to_string(),
                display_name: "billing_last_name".to_string(),
                field_type: enums::FieldType::UserBillingName,
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
            "billing.address.state".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.address.state".to_string(),
                display_name: "state".to_string(),
                field_type: enums::FieldType::UserAddressState,
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
            "billing.address.country".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.address.country".to_string(),
                display_name: "country".to_string(),
                field_type: enums::FieldType::UserAddressCountry {
                    options: vec!["ALL".to_string()],
                },
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
        ),
        (
            "billing.address.line2".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.address.line2".to_string(),
                display_name: "line2".to_string(),
                field_type: enums::FieldType::UserAddressLine2,
                value: None,
            },
        ),
        (
            "billing.phone.number".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.phone.number".to_string(),
                display_name: "phone_number".to_string(),
                field_type: enums::FieldType::UserPhoneNumber,
                value: None,
            },
        ),
        (
            "billing.phone.country_code".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.phone.country_code".to_string(),
                display_name: "dialing_code".to_string(),
                field_type: enums::FieldType::UserPhoneNumberCountryCode,
                value: None,
            },
        ),
        (
            "billing.email".to_string(),
            RequiredFieldInfo {
                required_field: "payment_method_data.billing.email".to_string(),
                display_name: "email".to_string(),
                field_type: enums::FieldType::UserEmailAddress,
                value: None,
            },
        ),
    ])
}

pub fn get_shipping_required_fields() -> HashMap<String, RequiredFieldInfo> {
    HashMap::from([
        (
            "shipping.address.first_name".to_string(),
            RequiredFieldInfo {
                required_field: "shipping.address.first_name".to_string(),
                display_name: "shipping_first_name".to_string(),
                field_type: enums::FieldType::UserShippingName,
                value: None,
            },
        ),
        (
            "shipping.address.last_name".to_string(),
            RequiredFieldInfo {
                required_field: "shipping.address.last_name".to_string(),
                display_name: "shipping_last_name".to_string(),
                field_type: enums::FieldType::UserShippingName,
                value: None,
            },
        ),
        (
            "shipping.address.city".to_string(),
            RequiredFieldInfo {
                required_field: "shipping.address.city".to_string(),
                display_name: "city".to_string(),
                field_type: enums::FieldType::UserShippingAddressCity,
                value: None,
            },
        ),
        (
            "shipping.address.state".to_string(),
            RequiredFieldInfo {
                required_field: "shipping.address.state".to_string(),
                display_name: "state".to_string(),
                field_type: enums::FieldType::UserShippingAddressState,
                value: None,
            },
        ),
        (
            "shipping.address.zip".to_string(),
            RequiredFieldInfo {
                required_field: "shipping.address.zip".to_string(),
                display_name: "zip".to_string(),
                field_type: enums::FieldType::UserShippingAddressPincode,
                value: None,
            },
        ),
        (
            "shipping.address.country".to_string(),
            RequiredFieldInfo {
                required_field: "shipping.address.country".to_string(),
                display_name: "country".to_string(),
                field_type: enums::FieldType::UserShippingAddressCountry {
                    options: vec!["ALL".to_string()],
                },
                value: None,
            },
        ),
        (
            "shipping.address.line1".to_string(),
            RequiredFieldInfo {
                required_field: "shipping.address.line1".to_string(),
                display_name: "line1".to_string(),
                field_type: enums::FieldType::UserShippingAddressLine1,
                value: None,
            },
        ),
        (
            "shipping.phone.number".to_string(),
            RequiredFieldInfo {
                required_field: "shipping.phone.number".to_string(),
                display_name: "phone_number".to_string(),
                field_type: enums::FieldType::UserPhoneNumber,
                value: None,
            },
        ),
        (
            "shipping.phone.country_code".to_string(),
            RequiredFieldInfo {
                required_field: "shipping.phone.country_code".to_string(),
                display_name: "dialing_code".to_string(),
                field_type: enums::FieldType::UserPhoneNumberCountryCode,
                value: None,
            },
        ),
        (
            "shipping.email".to_string(),
            RequiredFieldInfo {
                required_field: "shipping.email".to_string(),
                display_name: "email".to_string(),
                field_type: enums::FieldType::UserEmailAddress,
                value: None,
            },
        ),
    ])
}
