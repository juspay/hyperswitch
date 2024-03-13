use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, enums as storage_enums, schema::payment_methods};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable)]
#[diesel(table_name = payment_methods)]
pub struct PaymentMethod {
    pub id: i32,
    pub customer_id: String,
    pub merchant_id: String,
    pub payment_method_id: String,
    #[diesel(deserialize_as = super::OptionalDieselArray<storage_enums::Currency>)]
    pub accepted_currency: Option<Vec<storage_enums::Currency>>,
    pub scheme: Option<String>,
    pub token: Option<String>,
    pub cardholder_name: Option<Secret<String>>,
    pub issuer_name: Option<String>,
    pub issuer_country: Option<String>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub payer_country: Option<Vec<String>>,
    pub is_stored: Option<bool>,
    pub swift_code: Option<String>,
    pub direct_debit_token: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub payment_method: storage_enums::PaymentMethod,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<storage_enums::PaymentMethodIssuerCode>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub payment_method_data: Option<Encryption>,
    pub locker_id: Option<String>,
    pub last_used_at: PrimitiveDateTime,
    pub connector_mandate_details: Option<serde_json::Value>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::PaymentMethodStatus,
}

#[derive(Clone, Debug, Eq, PartialEq, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = payment_methods)]
pub struct PaymentMethodNew {
    pub customer_id: String,
    pub merchant_id: String,
    pub payment_method_id: String,
    pub payment_method: storage_enums::PaymentMethod,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub payment_method_issuer: Option<String>,
    pub payment_method_issuer_code: Option<storage_enums::PaymentMethodIssuerCode>,
    pub accepted_currency: Option<Vec<storage_enums::Currency>>,
    pub scheme: Option<String>,
    pub token: Option<String>,
    pub cardholder_name: Option<Secret<String>>,
    pub issuer_name: Option<String>,
    pub issuer_country: Option<String>,
    pub payer_country: Option<Vec<String>>,
    pub is_stored: Option<bool>,
    pub swift_code: Option<String>,
    pub direct_debit_token: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub payment_method_data: Option<Encryption>,
    pub locker_id: Option<String>,
    pub last_used_at: PrimitiveDateTime,
    pub connector_mandate_details: Option<serde_json::Value>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::PaymentMethodStatus,
}

impl Default for PaymentMethodNew {
    fn default() -> Self {
        let now = common_utils::date_time::now();

        Self {
            customer_id: String::default(),
            merchant_id: String::default(),
            payment_method_id: String::default(),
            locker_id: Option::default(),
            payment_method: storage_enums::PaymentMethod::default(),
            payment_method_type: Option::default(),
            payment_method_issuer: Option::default(),
            payment_method_issuer_code: Option::default(),
            accepted_currency: Option::default(),
            scheme: Option::default(),
            token: Option::default(),
            cardholder_name: Option::default(),
            issuer_name: Option::default(),
            issuer_country: Option::default(),
            payer_country: Option::default(),
            is_stored: Option::default(),
            swift_code: Option::default(),
            direct_debit_token: Option::default(),
            created_at: now,
            last_modified: now,
            metadata: Option::default(),
            payment_method_data: Option::default(),
            last_used_at: now,
            connector_mandate_details: Option::default(),
            customer_acceptance: Option::default(),
            status: storage_enums::PaymentMethodStatus::Active,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TokenizeCoreWorkflow {
    pub lookup_key: String,
    pub pm: storage_enums::PaymentMethod,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PaymentMethodUpdate {
    MetadataUpdate {
        metadata: Option<serde_json::Value>,
    },
    PaymentMethodDataUpdate {
        payment_method_data: Option<Encryption>,
    },
    LastUsedUpdate {
        last_used_at: PrimitiveDateTime,
    },
    StatusUpdate {
        status: Option<storage_enums::PaymentMethodStatus>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = payment_methods)]
pub struct PaymentMethodUpdateInternal {
    metadata: Option<serde_json::Value>,
    payment_method_data: Option<Encryption>,
    last_used_at: Option<PrimitiveDateTime>,
    status: Option<storage_enums::PaymentMethodStatus>,
}

impl PaymentMethodUpdateInternal {
    pub fn create_payment_method(self, source: PaymentMethod) -> PaymentMethod {
        let metadata = self.metadata.map(Secret::new);

        PaymentMethod { metadata, ..source }
    }
}

impl From<PaymentMethodUpdate> for PaymentMethodUpdateInternal {
    fn from(payment_method_update: PaymentMethodUpdate) -> Self {
        match payment_method_update {
            PaymentMethodUpdate::MetadataUpdate { metadata } => Self {
                metadata,
                payment_method_data: None,
                last_used_at: None,
                status: None,
            },
            PaymentMethodUpdate::PaymentMethodDataUpdate {
                payment_method_data,
            } => Self {
                metadata: None,
                payment_method_data,
                last_used_at: None,
                status: None,
            },
            PaymentMethodUpdate::LastUsedUpdate { last_used_at } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: Some(last_used_at),
                status: None,
            },
            PaymentMethodUpdate::StatusUpdate { status } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: None,
                status,
            },
        }
    }
}
