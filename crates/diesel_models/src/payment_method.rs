use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, enums as storage_enums, schema::payment_methods};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
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
    pub payment_method: Option<storage_enums::PaymentMethod>,
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
    pub network_transaction_id: Option<String>,
    pub client_secret: Option<String>,
    pub payment_method_billing_address: Option<Encryption>,
}

#[derive(
    Clone, Debug, Eq, PartialEq, Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize,
)]
#[diesel(table_name = payment_methods)]
pub struct PaymentMethodNew {
    pub customer_id: String,
    pub merchant_id: String,
    pub payment_method_id: String,
    pub payment_method: Option<storage_enums::PaymentMethod>,
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
    pub network_transaction_id: Option<String>,
    pub client_secret: Option<String>,
    pub payment_method_billing_address: Option<Encryption>,
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
    NetworkTransactionIdAndStatusUpdate {
        network_transaction_id: Option<String>,
        status: Option<storage_enums::PaymentMethodStatus>,
    },
    StatusUpdate {
        status: Option<storage_enums::PaymentMethodStatus>,
    },
    AdditionalDataUpdate {
        payment_method_data: Option<Encryption>,
        status: Option<storage_enums::PaymentMethodStatus>,
        locker_id: Option<String>,
        payment_method: Option<storage_enums::PaymentMethod>,
    },
    ConnectorMandateDetailsUpdate {
        connector_mandate_details: Option<serde_json::Value>,
    },
}

#[derive(
    Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay, Serialize, Deserialize,
)]
#[diesel(table_name = payment_methods)]
pub struct PaymentMethodUpdateInternal {
    metadata: Option<serde_json::Value>,
    payment_method_data: Option<Encryption>,
    last_used_at: Option<PrimitiveDateTime>,
    network_transaction_id: Option<String>,
    status: Option<storage_enums::PaymentMethodStatus>,
    locker_id: Option<String>,
    payment_method: Option<storage_enums::PaymentMethod>,
    connector_mandate_details: Option<serde_json::Value>,
}

impl PaymentMethodUpdateInternal {
    pub fn create_payment_method(self, source: PaymentMethod) -> PaymentMethod {
        let metadata = self.metadata.map(Secret::new);

        PaymentMethod { metadata, ..source }
    }

    pub fn apply_changeset(self, source: PaymentMethod) -> PaymentMethod {
        let Self {
            metadata,
            payment_method_data,
            last_used_at,
            network_transaction_id,
            status,
            connector_mandate_details,
            ..
        } = self;

        PaymentMethod {
            metadata: metadata.map_or(source.metadata, |v| Some(v.into())),
            payment_method_data: payment_method_data.map_or(source.payment_method_data, Some),
            last_used_at: last_used_at.unwrap_or(source.last_used_at),
            network_transaction_id: network_transaction_id
                .map_or(source.network_transaction_id, Some),
            status: status.unwrap_or(source.status),
            connector_mandate_details: connector_mandate_details
                .map_or(source.connector_mandate_details, Some),
            ..source
        }
    }
}

impl From<PaymentMethodUpdate> for PaymentMethodUpdateInternal {
    fn from(payment_method_update: PaymentMethodUpdate) -> Self {
        match payment_method_update {
            PaymentMethodUpdate::MetadataUpdate { metadata } => Self {
                metadata,
                payment_method_data: None,
                last_used_at: None,
                network_transaction_id: None,
                status: None,
                locker_id: None,
                payment_method: None,
                connector_mandate_details: None,
            },
            PaymentMethodUpdate::PaymentMethodDataUpdate {
                payment_method_data,
            } => Self {
                metadata: None,
                payment_method_data,
                last_used_at: None,
                network_transaction_id: None,
                status: None,
                locker_id: None,
                payment_method: None,
                connector_mandate_details: None,
            },
            PaymentMethodUpdate::LastUsedUpdate { last_used_at } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: Some(last_used_at),
                network_transaction_id: None,
                status: None,
                locker_id: None,
                payment_method: None,
                connector_mandate_details: None,
            },
            PaymentMethodUpdate::NetworkTransactionIdAndStatusUpdate {
                network_transaction_id,
                status,
            } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: None,
                network_transaction_id,
                status,
                locker_id: None,
                payment_method: None,
                connector_mandate_details: None,
            },
            PaymentMethodUpdate::StatusUpdate { status } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: None,
                network_transaction_id: None,
                status,
                locker_id: None,
                payment_method: None,
                connector_mandate_details: None,
            },
            PaymentMethodUpdate::AdditionalDataUpdate {
                payment_method_data,
                status,
                locker_id,
                payment_method,
            } => Self {
                metadata: None,
                payment_method_data,
                last_used_at: None,
                network_transaction_id: None,
                status,
                locker_id,
                payment_method,
                connector_mandate_details: None,
            },
            PaymentMethodUpdate::ConnectorMandateDetailsUpdate {
                connector_mandate_details,
            } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: None,
                status: None,
                locker_id: None,
                payment_method: None,
                connector_mandate_details,
                network_transaction_id: None,
            },
        }
    }
}

impl From<&PaymentMethodNew> for PaymentMethod {
    fn from(payment_method_new: &PaymentMethodNew) -> Self {
        Self {
            id: 0i32,
            customer_id: payment_method_new.customer_id.clone(),
            merchant_id: payment_method_new.merchant_id.clone(),
            payment_method_id: payment_method_new.payment_method_id.clone(),
            locker_id: payment_method_new.locker_id.clone(),
            accepted_currency: payment_method_new.accepted_currency.clone(),
            scheme: payment_method_new.scheme.clone(),
            token: payment_method_new.token.clone(),
            cardholder_name: payment_method_new.cardholder_name.clone(),
            issuer_name: payment_method_new.issuer_name.clone(),
            issuer_country: payment_method_new.issuer_country.clone(),
            payer_country: payment_method_new.payer_country.clone(),
            is_stored: payment_method_new.is_stored,
            swift_code: payment_method_new.swift_code.clone(),
            direct_debit_token: payment_method_new.direct_debit_token.clone(),
            created_at: payment_method_new.created_at,
            last_modified: payment_method_new.last_modified,
            payment_method: payment_method_new.payment_method,
            payment_method_type: payment_method_new.payment_method_type,
            payment_method_issuer: payment_method_new.payment_method_issuer.clone(),
            payment_method_issuer_code: payment_method_new.payment_method_issuer_code,
            metadata: payment_method_new.metadata.clone(),
            payment_method_data: payment_method_new.payment_method_data.clone(),
            last_used_at: payment_method_new.last_used_at,
            connector_mandate_details: payment_method_new.connector_mandate_details.clone(),
            customer_acceptance: payment_method_new.customer_acceptance.clone(),
            status: payment_method_new.status,
            network_transaction_id: payment_method_new.network_transaction_id.clone(),
            client_secret: payment_method_new.client_secret.clone(),
            payment_method_billing_address: payment_method_new
                .payment_method_billing_address
                .clone(),
        }
    }
}
