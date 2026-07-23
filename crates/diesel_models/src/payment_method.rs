use std::collections::HashMap;

use common_enums::MerchantStorageScheme;
use common_utils::{
    encryption::Encryption,
    errors::{CustomResult, ParsingError},
    pii,
};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use error_stack::ResultExt;
#[cfg(feature = "v1")]
use hyperswitch_masking::ExposeInterface;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

#[cfg(feature = "v1")]
use crate::{enums as storage_enums, schema::payment_methods};
#[cfg(feature = "v2")]
use crate::{enums as storage_enums, schema_v2::payment_methods};

#[cfg(feature = "v1")]
#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Serialize, Deserialize,
)]
#[diesel(table_name = payment_methods, primary_key(payment_method_id), check_for_backend(diesel::pg::Pg))]
pub struct PaymentMethod {
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
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
    pub updated_by: Option<String>,
    pub version: common_enums::ApiVersion,
    pub network_token_requestor_reference_id: Option<String>,
    pub network_token_locker_id: Option<String>,
    pub network_token_payment_method_data: Option<Encryption>,
    pub external_vault_source: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub vault_type: Option<storage_enums::VaultType>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
    pub customer_details: Option<Encryption>,
    pub locker_fingerprint_id: Option<String>,
    pub network_tokenization_data: Option<Encryption>,
    // Compatibility-only field: backfilled by modular-compat PT to align with v2 identifier semantics.
    // Do not use this column in v1 business logic.
    pub id: Option<String>,
    // Compatibility-only field: backfilled by modular-compat PT for v2 interoperability.
    // Do not use this column in v1 business logic.
    pub payment_method_type_v2: Option<storage_enums::PaymentMethod>,
    // Compatibility-only field: backfilled by modular-compat PT for v2 interoperability.
    // Do not use this column in v1 business logic.
    pub payment_method_subtype: Option<String>,
    pub network_transaction_link_id: Option<String>,
    pub compatibility_updated_at: Option<PrimitiveDateTime>,
    // Connector-specific payment method details returned during a payment.
    pub connector_payment_method_details: Option<pii::SecretSerdeValue>,
    pub auxiliary_fingerprint_id: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Serialize, Deserialize)]
#[diesel(table_name = payment_methods, primary_key(id), check_for_backend(diesel::pg::Pg))]
pub struct PaymentMethod {
    //customer_id is made optional in v2 to accommodate guest checkout flow where customer id is not present.
    //customer_id should only be none in case of guest checkout flow as volatile pm are stored in redis, for all other cases it should be present to maintain the db persistency
    pub customer_id: Option<common_utils::id_type::GlobalCustomerId>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_method_id: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub payment_method: Option<storage_enums::PaymentMethod>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub payment_method_data: Option<Encryption>,
    pub locker_id: Option<String>,
    pub last_used_at: PrimitiveDateTime,
    pub connector_mandate_details: Option<CommonMandateReference>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::PaymentMethodStatus,
    pub network_transaction_id: Option<Secret<String>>,
    pub client_secret: Option<String>,
    pub payment_method_billing_address: Option<Encryption>,
    pub updated_by: Option<String>,
    pub version: common_enums::ApiVersion,
    pub network_token_requestor_reference_id: Option<String>,
    pub network_token_locker_id: Option<String>,
    pub network_token_payment_method_data: Option<Encryption>,
    pub external_vault_source: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub vault_type: Option<storage_enums::VaultType>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
    pub customer_details: Option<Encryption>,
    pub locker_fingerprint_id: Option<String>,
    pub network_tokenization_data: Option<Encryption>,
    pub id: common_utils::id_type::GlobalPaymentMethodId,
    pub payment_method_type_v2: Option<storage_enums::PaymentMethod>,
    pub payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    pub network_transaction_link_id: Option<Secret<String>>,
    pub compatibility_updated_at: Option<PrimitiveDateTime>,
    pub auxiliary_fingerprint_id: Option<String>,
    pub external_vault_token_data: Option<Encryption>,
}

impl PaymentMethod {
    #[cfg(feature = "v1")]
    pub fn get_id(&self) -> &String {
        &self.payment_method_id
    }

    #[cfg(feature = "v2")]
    pub fn get_id(&self) -> &common_utils::id_type::GlobalPaymentMethodId {
        &self.id
    }
}

#[cfg(feature = "v1")]
#[derive(
    Clone, Debug, Eq, PartialEq, Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize,
)]
#[diesel(table_name = payment_methods)]
pub struct PaymentMethodNew {
    pub customer_id: common_utils::id_type::CustomerId,
    pub merchant_id: common_utils::id_type::MerchantId,
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
    pub network_transaction_link_id: Option<String>,
    pub client_secret: Option<String>,
    pub payment_method_billing_address: Option<Encryption>,
    pub updated_by: Option<String>,
    pub version: common_enums::ApiVersion,
    pub network_token_requestor_reference_id: Option<String>,
    pub network_token_locker_id: Option<String>,
    pub network_token_payment_method_data: Option<Encryption>,
    pub external_vault_source: Option<common_utils::id_type::MerchantConnectorAccountId>,
    pub vault_type: Option<storage_enums::VaultType>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
    pub customer_details: Option<Encryption>,
    pub locker_fingerprint_id: Option<String>,
    pub network_tokenization_data: Option<Encryption>,
    pub id: Option<String>,
    pub compatibility_updated_at: Option<PrimitiveDateTime>,
    // Connector-specific payment method details returned during a payment.
    pub connector_payment_method_details: Option<pii::SecretSerdeValue>,
    pub auxiliary_fingerprint_id: Option<String>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize)]
#[diesel(table_name = payment_methods)]
pub struct PaymentMethodNew {
    pub customer_id: Option<common_utils::id_type::GlobalCustomerId>,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_method_id: Option<String>,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub payment_method: Option<storage_enums::PaymentMethod>,
    pub payment_method_type: Option<storage_enums::PaymentMethodType>,
    pub payment_method_data: Option<Encryption>,
    pub locker_id: Option<String>,
    pub last_used_at: PrimitiveDateTime,
    pub connector_mandate_details: Option<CommonMandateReference>,
    pub customer_acceptance: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::PaymentMethodStatus,
    pub network_transaction_id: Option<String>,
    pub network_transaction_link_id: Option<String>,
    pub client_secret: Option<String>,
    pub payment_method_billing_address: Option<Encryption>,
    pub updated_by: Option<String>,
    pub version: common_enums::ApiVersion,
    pub network_token_requestor_reference_id: Option<String>,
    pub network_token_locker_id: Option<String>,
    pub network_token_payment_method_data: Option<Encryption>,
    pub external_vault_token_data: Option<Encryption>,
    pub locker_fingerprint_id: Option<String>,
    pub auxiliary_fingerprint_id: Option<String>,
    pub payment_method_type_v2: Option<storage_enums::PaymentMethod>,
    pub payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    pub id: common_utils::id_type::GlobalPaymentMethodId,
    pub vault_type: Option<storage_enums::VaultType>,
    pub created_by: Option<String>,
    pub last_modified_by: Option<String>,
    pub customer_details: Option<Encryption>,
    pub compatibility_updated_at: Option<PrimitiveDateTime>,
    pub external_vault_source: Option<common_utils::id_type::MerchantConnectorAccountId>,
}

impl PaymentMethodNew {
    pub fn update_storage_scheme(&mut self, storage_scheme: MerchantStorageScheme) {
        self.updated_by = Some(storage_scheme.to_string());
    }

    #[cfg(feature = "v1")]
    pub fn get_id(&self) -> &String {
        &self.payment_method_id
    }

    #[cfg(feature = "v2")]
    pub fn get_id(&self) -> &common_utils::id_type::GlobalPaymentMethodId {
        &self.id
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct TokenizeCoreWorkflow {
    pub lookup_key: String,
    pub pm: storage_enums::PaymentMethod,
}

#[cfg(feature = "v1")]
#[derive(Debug, Serialize, Deserialize)]
pub enum PaymentMethodUpdate {
    MetadataUpdateAndLastUsed {
        metadata: Option<serde_json::Value>,
        last_used_at: PrimitiveDateTime,
        last_modified_by: Option<String>,
    },
    UpdatePaymentMethodDataAndLastUsed {
        payment_method_data: Option<Encryption>,
        scheme: Option<String>,
        last_used_at: PrimitiveDateTime,
        last_modified_by: Option<String>,
    },
    PaymentMethodDataUpdate {
        payment_method_data: Option<Encryption>,
        last_modified_by: Option<String>,
    },
    LastUsedUpdate {
        last_used_at: PrimitiveDateTime,
    },
    NetworkTransactionIdAndStatusUpdate {
        network_transaction_id: Option<String>,
        network_transaction_link_id: Option<String>,
        status: Option<storage_enums::PaymentMethodStatus>,
        last_modified_by: Option<String>,
    },
    NetworkTransactionLinkIdUpdate {
        network_transaction_link_id: Option<String>,
        last_modified_by: Option<String>,
    },
    StatusUpdate {
        status: Option<storage_enums::PaymentMethodStatus>,
        last_modified_by: Option<String>,
    },
    AdditionalDataUpdate {
        payment_method_data: Option<Encryption>,
        status: Option<storage_enums::PaymentMethodStatus>,
        locker_id: Option<String>,
        locker_fingerprint_id: Option<String>,
        payment_method: Option<storage_enums::PaymentMethod>,
        payment_method_type: Option<storage_enums::PaymentMethodType>,
        payment_method_issuer: Option<String>,
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: Option<Encryption>,
        last_modified_by: Option<String>,
        metadata: Option<serde_json::Value>,
        last_used_at: Option<PrimitiveDateTime>,
        connector_mandate_details: Option<Box<serde_json::Value>>,
        network_tokenization_data: Option<Encryption>,
        connector_payment_method_details: Box<Option<pii::SecretSerdeValue>>,
    },
    ConnectorMandateDetailsUpdate {
        connector_mandate_details: Option<pii::SecretSerdeValue>,
        last_modified_by: Option<String>,
    },
    NetworkTokenDataUpdate {
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: Option<Encryption>,
        network_tokenization_data: Option<Encryption>,
        last_modified_by: Option<String>,
    },
    ConnectorNetworkTransactionIdAndMandateDetailsUpdate {
        connector_mandate_details: Option<pii::SecretSerdeValue>,
        network_transaction_id: Option<Secret<String>>,
        last_modified_by: Option<String>,
    },
    PaymentMethodBatchUpdate {
        connector_mandate_details: Option<pii::SecretSerdeValue>,
        network_transaction_id: Option<String>,
        network_transaction_link_id: Option<String>,
        status: Option<storage_enums::PaymentMethodStatus>,
        payment_method_data: Option<Encryption>,
        last_modified_by: Option<String>,
    },
    // Compatibility-only update used by modular-compat PT.
    // Do not use this for normal v1 payment method updates.
    PopulateModularCompatFields {
        id: String,
        payment_method_type_v2: Option<storage_enums::PaymentMethod>,
        payment_method_subtype: Option<storage_enums::PaymentMethodType>,
        connector_mandate_details: Option<serde_json::Value>,
        locker_fingerprint_id: Option<String>,
        auxiliary_fingerprint_id: Option<String>,
        last_modified_by: Option<String>,
    },
    // Compatibility-only update used by modular backward-compat PT.
    // Do not use this for normal v1 payment method updates.
    PopulateLegacyCompatFields {
        payment_method: Option<storage_enums::PaymentMethod>,
        payment_method_type: Option<storage_enums::PaymentMethodType>,
        connector_mandate_details: Option<serde_json::Value>,
        last_modified_by: Option<String>,
    },
}

#[cfg(feature = "v2")]
#[derive(Debug, Serialize, Deserialize)]
pub enum PaymentMethodUpdate {
    UpdatePaymentMethodDataAndLastUsed {
        payment_method_data: Option<Encryption>,
        scheme: Option<String>,
        last_used_at: PrimitiveDateTime,
        last_modified_by: Option<String>,
    },
    PaymentMethodDataUpdate {
        payment_method_data: Option<Encryption>,
        last_modified_by: Option<String>,
    },
    LastUsedUpdate {
        last_used_at: PrimitiveDateTime,
    },
    NetworkTransactionIdAndStatusUpdate {
        network_transaction_id: Option<Secret<String>>,
        network_transaction_link_id: Option<Secret<String>>,
        status: Option<storage_enums::PaymentMethodStatus>,
        last_modified_by: Option<String>,
    },
    StatusUpdate {
        status: Option<storage_enums::PaymentMethodStatus>,
        last_modified_by: Option<String>,
    },
    GenericUpdate {
        payment_method_data: Option<Encryption>,
        status: Option<storage_enums::PaymentMethodStatus>,
        locker_id: Option<String>,
        payment_method_type_v2: Option<storage_enums::PaymentMethod>,
        payment_method_subtype: Option<storage_enums::PaymentMethodType>,
        network_token_requestor_reference_id: Option<String>,
        network_token_locker_id: Option<String>,
        network_token_payment_method_data: Option<Encryption>,
        locker_fingerprint_id: Option<String>,
        connector_mandate_details: Box<Option<CommonMandateReference>>,
        external_vault_source: Option<common_utils::id_type::MerchantConnectorAccountId>,
        network_transaction_id: Option<Secret<String>>,
        network_transaction_link_id: Option<Secret<String>>,
        last_modified_by: Option<String>,
    },
    ConnectorMandateDetailsUpdate {
        connector_mandate_details: Option<CommonMandateReference>,
        last_modified_by: Option<String>,
    },
    StatusAndFingerprintUpdate {
        status: Option<storage_enums::PaymentMethodStatus>,
        locker_fingerprint_id: Option<String>,
        last_modified_by: Option<String>,
    },
    // Compatibility-only update used by modular backward-compat inline/PT.
    // Do not use this for normal v2 payment method updates.
    PopulateLegacyCompatFields {
        payment_method: Option<storage_enums::PaymentMethod>,
        payment_method_type: Option<storage_enums::PaymentMethodType>,
        connector_mandate_details: Option<CommonMandateReference>,
        last_modified_by: Option<String>,
    },
}

impl PaymentMethodUpdate {
    pub fn convert_to_payment_method_update(
        self,
        storage_scheme: MerchantStorageScheme,
    ) -> PaymentMethodUpdateInternal {
        let mut update_internal: PaymentMethodUpdateInternal = self.into();
        update_internal.updated_by = Some(storage_scheme.to_string());
        update_internal
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay, Serialize, Deserialize)]
#[diesel(table_name = payment_methods)]
#[router_derive::apply_changeset(target = PaymentMethod)]
pub struct PaymentMethodUpdateInternal {
    payment_method_data: Option<Encryption>,
    last_used_at: Option<PrimitiveDateTime>,
    network_transaction_id: Option<Secret<String>>,
    network_transaction_link_id: Option<Secret<String>>,
    status: Option<storage_enums::PaymentMethodStatus>,
    locker_id: Option<String>,
    payment_method: Option<storage_enums::PaymentMethod>,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    payment_method_type_v2: Option<storage_enums::PaymentMethod>,
    connector_mandate_details: Option<CommonMandateReference>,
    updated_by: Option<String>,
    payment_method_subtype: Option<storage_enums::PaymentMethodType>,
    last_modified: PrimitiveDateTime,
    network_token_requestor_reference_id: Option<String>,
    network_token_locker_id: Option<String>,
    network_token_payment_method_data: Option<Encryption>,
    locker_fingerprint_id: Option<String>,
    external_vault_source: Option<common_utils::id_type::MerchantConnectorAccountId>,
    last_modified_by: Option<String>,
    customer_details: Option<Encryption>,
    compatibility_updated_at: Option<PrimitiveDateTime>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay, Serialize, Deserialize)]
// Diesel derive assumes `id` as primary key unless explicitly configured.
// This changeset must update the nullable `payment_methods.id` column, so we
// pin the actual primary key to `payment_method_id`.
// Ref: https://diesel.rs/guides/all-about-updates/
#[diesel(table_name = payment_methods)]
#[diesel(primary_key(payment_method_id))]
#[router_derive::apply_changeset(target = PaymentMethod)]
pub struct PaymentMethodUpdateInternal {
    metadata: Option<pii::SecretSerdeValue>,
    payment_method_data: Option<Encryption>,
    last_used_at: Option<PrimitiveDateTime>,
    network_transaction_id: Option<String>,
    network_transaction_link_id: Option<String>,
    status: Option<storage_enums::PaymentMethodStatus>,
    locker_id: Option<String>,
    locker_fingerprint_id: Option<String>,
    network_token_requestor_reference_id: Option<String>,
    payment_method: Option<storage_enums::PaymentMethod>,
    connector_mandate_details: Option<serde_json::Value>,
    updated_by: Option<String>,
    payment_method_type: Option<storage_enums::PaymentMethodType>,
    payment_method_issuer: Option<String>,
    last_modified: PrimitiveDateTime,
    network_token_locker_id: Option<String>,
    network_token_payment_method_data: Option<Encryption>,
    scheme: Option<String>,
    last_modified_by: Option<String>,
    customer_details: Option<Encryption>,
    network_tokenization_data: Option<Encryption>,
    payment_method_type_v2: Option<storage_enums::PaymentMethod>,
    payment_method_subtype: Option<String>,
    id: Option<String>,
    version: Option<common_enums::ApiVersion>,
    compatibility_updated_at: Option<PrimitiveDateTime>,
    connector_payment_method_details: Option<pii::SecretSerdeValue>,
    auxiliary_fingerprint_id: Option<String>,
}

#[cfg(feature = "v1")]
impl From<PaymentMethodUpdate> for PaymentMethodUpdateInternal {
    fn from(payment_method_update: PaymentMethodUpdate) -> Self {
        match payment_method_update {
            PaymentMethodUpdate::MetadataUpdateAndLastUsed {
                metadata,
                last_used_at,
                last_modified_by,
            } => Self {
                metadata: metadata.map(Secret::new),
                payment_method_data: None,
                last_used_at: Some(last_used_at),
                network_transaction_id: None,
                network_transaction_link_id: None,
                status: None,
                locker_id: None,
                locker_fingerprint_id: None,
                network_token_requestor_reference_id: None,
                payment_method: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                last_modified: common_utils::date_time::now(),
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                scheme: None,
                last_modified_by,
                customer_details: None,
                network_tokenization_data: None,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::PaymentMethodDataUpdate {
                payment_method_data,
                last_modified_by,
            } => Self {
                metadata: None,
                payment_method_data,
                last_used_at: None,
                network_transaction_id: None,
                network_transaction_link_id: None,
                status: None,
                locker_id: None,
                locker_fingerprint_id: None,
                network_token_requestor_reference_id: None,
                payment_method: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                last_modified: common_utils::date_time::now(),
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                scheme: None,
                last_modified_by,
                customer_details: None,
                network_tokenization_data: None,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::LastUsedUpdate { last_used_at } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: Some(last_used_at),
                network_transaction_id: None,
                network_transaction_link_id: None,
                status: None,
                locker_id: None,
                locker_fingerprint_id: None,
                network_token_requestor_reference_id: None,
                payment_method: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                last_modified: common_utils::date_time::now(),
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                scheme: None,
                last_modified_by: None,
                customer_details: None,
                network_tokenization_data: None,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::UpdatePaymentMethodDataAndLastUsed {
                payment_method_data,
                scheme,
                last_used_at,
                last_modified_by,
            } => Self {
                metadata: None,
                payment_method_data,
                last_used_at: Some(last_used_at),
                network_transaction_id: None,
                network_transaction_link_id: None,
                status: None,
                locker_id: None,
                locker_fingerprint_id: None,
                network_token_requestor_reference_id: None,
                payment_method: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                last_modified: common_utils::date_time::now(),
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                scheme,
                last_modified_by,
                customer_details: None,
                network_tokenization_data: None,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::NetworkTransactionIdAndStatusUpdate {
                network_transaction_id,
                network_transaction_link_id,
                status,
                last_modified_by,
            } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: None,
                network_transaction_id,
                network_transaction_link_id,
                status,
                locker_id: None,
                locker_fingerprint_id: None,
                network_token_requestor_reference_id: None,
                payment_method: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                last_modified: common_utils::date_time::now(),
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                scheme: None,
                last_modified_by,
                customer_details: None,
                network_tokenization_data: None,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::NetworkTransactionLinkIdUpdate {
                network_transaction_link_id,
                last_modified_by,
            } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: None,
                network_transaction_id: None,
                network_transaction_link_id,
                status: None,
                locker_id: None,
                locker_fingerprint_id: None,
                network_token_requestor_reference_id: None,
                payment_method: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                last_modified: common_utils::date_time::now(),
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                scheme: None,
                last_modified_by,
                customer_details: None,
                network_tokenization_data: None,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::StatusUpdate {
                status,
                last_modified_by,
            } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: None,
                network_transaction_id: None,
                network_transaction_link_id: None,
                status,
                locker_id: None,
                locker_fingerprint_id: None,
                network_token_requestor_reference_id: None,
                payment_method: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                last_modified: common_utils::date_time::now(),
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                scheme: None,
                last_modified_by,
                customer_details: None,
                network_tokenization_data: None,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::AdditionalDataUpdate {
                payment_method_data,
                status,
                locker_id,
                locker_fingerprint_id,
                network_token_requestor_reference_id,
                payment_method,
                payment_method_type,
                payment_method_issuer,
                network_token_locker_id,
                network_token_payment_method_data,
                last_modified_by,
                metadata,
                last_used_at,
                connector_mandate_details,
                network_tokenization_data,
                connector_payment_method_details,
            } => Self {
                metadata: metadata.map(Secret::new),
                payment_method_data,
                last_used_at,
                network_transaction_id: None,
                network_transaction_link_id: None,
                status,
                locker_id,
                locker_fingerprint_id,
                network_token_requestor_reference_id,
                payment_method,
                connector_mandate_details: connector_mandate_details
                    .map(|mandate_details| *mandate_details),
                connector_payment_method_details: *connector_payment_method_details,
                updated_by: None,
                payment_method_issuer,
                payment_method_type,
                last_modified: common_utils::date_time::now(),
                network_token_locker_id,
                network_token_payment_method_data,
                scheme: None,
                last_modified_by,
                customer_details: None,
                network_tokenization_data,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
            },
            PaymentMethodUpdate::ConnectorMandateDetailsUpdate {
                connector_mandate_details,
                last_modified_by,
            } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: None,
                status: None,
                locker_id: None,
                locker_fingerprint_id: None,
                network_token_requestor_reference_id: None,
                payment_method: None,
                connector_mandate_details: connector_mandate_details.map(|v| v.expose()),
                network_transaction_id: None,
                network_transaction_link_id: None,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                last_modified: common_utils::date_time::now(),
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                scheme: None,
                last_modified_by,
                customer_details: None,
                network_tokenization_data: None,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::NetworkTokenDataUpdate {
                network_token_requestor_reference_id,
                network_token_locker_id,
                network_token_payment_method_data,
                network_tokenization_data,
                last_modified_by,
            } => Self {
                metadata: None,
                payment_method_data: None,
                last_used_at: None,
                status: None,
                locker_id: None,
                locker_fingerprint_id: None,
                payment_method: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                last_modified: common_utils::date_time::now(),
                network_transaction_id: None,
                network_transaction_link_id: None,
                network_token_requestor_reference_id,
                network_token_locker_id,
                network_token_payment_method_data,
                scheme: None,
                last_modified_by,
                customer_details: None,
                network_tokenization_data,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::ConnectorNetworkTransactionIdAndMandateDetailsUpdate {
                connector_mandate_details,
                network_transaction_id,
                last_modified_by,
            } => Self {
                connector_mandate_details: connector_mandate_details
                    .map(|mandate_details| mandate_details.expose()),
                network_transaction_id: network_transaction_id.map(|txn_id| txn_id.expose()),
                network_transaction_link_id: None,
                last_modified: common_utils::date_time::now(),
                status: None,
                metadata: None,
                payment_method_data: None,
                last_used_at: None,
                locker_id: None,
                locker_fingerprint_id: None,
                payment_method: None,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                network_token_requestor_reference_id: None,
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                scheme: None,
                last_modified_by,
                customer_details: None,
                network_tokenization_data: None,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::PaymentMethodBatchUpdate {
                connector_mandate_details,
                network_transaction_id,
                network_transaction_link_id,
                status,
                payment_method_data,
                last_modified_by,
            } => Self {
                metadata: None,
                last_used_at: None,
                status,
                locker_id: None,
                locker_fingerprint_id: None,
                network_token_requestor_reference_id: None,
                payment_method: None,
                connector_mandate_details: connector_mandate_details
                    .map(|mandate_details| mandate_details.expose()),
                network_transaction_id,
                network_transaction_link_id,
                updated_by: None,
                payment_method_issuer: None,
                payment_method_type: None,
                last_modified: common_utils::date_time::now(),
                network_token_locker_id: None,
                network_token_payment_method_data: None,
                scheme: None,
                payment_method_data,
                last_modified_by,
                customer_details: None,
                network_tokenization_data: None,
                id: None,
                payment_method_type_v2: None,
                payment_method_subtype: None,
                version: None,
                compatibility_updated_at: None,
                auxiliary_fingerprint_id: None,
                connector_payment_method_details: None,
            },
            PaymentMethodUpdate::PopulateModularCompatFields {
                id,
                payment_method_type_v2,
                payment_method_subtype,
                connector_mandate_details,
                locker_fingerprint_id,
                auxiliary_fingerprint_id,
                last_modified_by,
            } => {
                let now = common_utils::date_time::now();

                Self {
                    metadata: None,
                    payment_method_data: None,
                    last_used_at: None,
                    network_transaction_id: None,
                    network_transaction_link_id: None,
                    status: None,
                    locker_id: None,
                    locker_fingerprint_id,
                    network_token_requestor_reference_id: None,
                    payment_method: None,
                    connector_mandate_details,
                    updated_by: None,
                    payment_method_issuer: None,
                    payment_method_type: None,
                    last_modified: now,
                    network_token_locker_id: None,
                    network_token_payment_method_data: None,
                    scheme: None,
                    last_modified_by,
                    customer_details: None,
                    network_tokenization_data: None,
                    payment_method_type_v2,
                    payment_method_subtype: payment_method_subtype.map(|x| x.to_string()),
                    id: Some(id),
                    version: Some(common_enums::ApiVersion::V2),
                    compatibility_updated_at: Some(now),
                    auxiliary_fingerprint_id,
                    connector_payment_method_details: None,
                }
            }
            PaymentMethodUpdate::PopulateLegacyCompatFields {
                payment_method,
                payment_method_type,
                connector_mandate_details,
                last_modified_by,
            } => {
                let now = common_utils::date_time::now();

                Self {
                    metadata: None,
                    payment_method_data: None,
                    last_used_at: None,
                    network_transaction_id: None,
                    network_transaction_link_id: None,
                    status: None,
                    locker_id: None,
                    locker_fingerprint_id: None,
                    network_token_requestor_reference_id: None,
                    payment_method,
                    connector_mandate_details,
                    updated_by: None,
                    payment_method_issuer: None,
                    payment_method_type,
                    last_modified: now,
                    network_token_locker_id: None,
                    network_token_payment_method_data: None,
                    scheme: None,
                    last_modified_by,
                    customer_details: None,
                    network_tokenization_data: None,
                    payment_method_type_v2: None,
                    payment_method_subtype: None,
                    id: None,
                    version: None,
                    compatibility_updated_at: Some(now),
                    auxiliary_fingerprint_id: None,
                    connector_payment_method_details: None,
                }
            }
        }
    }
}

#[cfg(feature = "v2")]
impl From<PaymentMethodUpdate> for PaymentMethodUpdateInternal {
    fn from(payment_method_update: PaymentMethodUpdate) -> Self {
        let now = common_utils::date_time::now();

        match payment_method_update {
            PaymentMethodUpdate::PaymentMethodDataUpdate {
                payment_method_data,
                last_modified_by,
            } => Self {
                payment_method_data,
                last_used_at: None,
                network_transaction_id: None,
                network_transaction_link_id: None,
                status: None,
                locker_id: None,
                payment_method: None,
                payment_method_type: None,
                payment_method_type_v2: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_subtype: None,
                last_modified: now,
                network_token_locker_id: None,
                network_token_requestor_reference_id: None,
                network_token_payment_method_data: None,
                locker_fingerprint_id: None,
                external_vault_source: None,
                last_modified_by,
                customer_details: None,
                compatibility_updated_at: Some(now),
            },
            PaymentMethodUpdate::LastUsedUpdate { last_used_at } => Self {
                payment_method_data: None,
                last_used_at: Some(last_used_at),
                network_transaction_id: None,
                network_transaction_link_id: None,
                status: None,
                locker_id: None,
                payment_method: None,
                payment_method_type: None,
                payment_method_type_v2: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_subtype: None,
                last_modified: now,
                network_token_locker_id: None,
                network_token_requestor_reference_id: None,
                network_token_payment_method_data: None,
                locker_fingerprint_id: None,
                external_vault_source: None,
                last_modified_by: None,
                customer_details: None,
                compatibility_updated_at: Some(now),
            },
            PaymentMethodUpdate::UpdatePaymentMethodDataAndLastUsed {
                payment_method_data,
                last_used_at,
                last_modified_by,
                ..
            } => Self {
                payment_method_data,
                last_used_at: Some(last_used_at),
                network_transaction_id: None,
                network_transaction_link_id: None,
                status: None,
                locker_id: None,
                payment_method: None,
                payment_method_type: None,
                payment_method_type_v2: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_subtype: None,
                last_modified: now,
                network_token_locker_id: None,
                network_token_requestor_reference_id: None,
                network_token_payment_method_data: None,
                locker_fingerprint_id: None,
                external_vault_source: None,
                last_modified_by,
                customer_details: None,
                compatibility_updated_at: Some(now),
            },
            PaymentMethodUpdate::NetworkTransactionIdAndStatusUpdate {
                network_transaction_id,
                network_transaction_link_id,
                status,
                last_modified_by,
            } => Self {
                payment_method_data: None,
                last_used_at: None,
                network_transaction_id,
                network_transaction_link_id,
                status,
                locker_id: None,
                payment_method: None,
                payment_method_type: None,
                payment_method_type_v2: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_subtype: None,
                last_modified: now,
                network_token_locker_id: None,
                network_token_requestor_reference_id: None,
                network_token_payment_method_data: None,
                locker_fingerprint_id: None,
                external_vault_source: None,
                last_modified_by,
                customer_details: None,
                compatibility_updated_at: Some(now),
            },
            PaymentMethodUpdate::StatusUpdate {
                status,
                last_modified_by,
            } => Self {
                payment_method_data: None,
                last_used_at: None,
                network_transaction_id: None,
                network_transaction_link_id: None,
                status,
                locker_id: None,
                payment_method: None,
                payment_method_type: None,
                payment_method_type_v2: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_subtype: None,
                last_modified: now,
                network_token_locker_id: None,
                network_token_requestor_reference_id: None,
                network_token_payment_method_data: None,
                locker_fingerprint_id: None,
                external_vault_source: None,
                last_modified_by,
                customer_details: None,
                compatibility_updated_at: Some(now),
            },
            PaymentMethodUpdate::GenericUpdate {
                payment_method_data,
                status,
                locker_id,
                payment_method_type_v2,
                payment_method_subtype,
                network_token_requestor_reference_id,
                network_token_locker_id,
                network_token_payment_method_data,
                locker_fingerprint_id,
                connector_mandate_details,
                external_vault_source,
                network_transaction_id,
                network_transaction_link_id,
                last_modified_by,
            } => Self {
                payment_method_data,
                last_used_at: None,
                status,
                locker_id,
                payment_method: None,
                payment_method_type: None,
                payment_method_type_v2,
                connector_mandate_details: *connector_mandate_details,
                updated_by: None,
                payment_method_subtype,
                last_modified: now,
                network_token_requestor_reference_id,
                network_token_locker_id,
                network_token_payment_method_data,
                locker_fingerprint_id,
                external_vault_source,
                network_transaction_id,
                network_transaction_link_id,
                last_modified_by,
                customer_details: None,
                compatibility_updated_at: Some(now),
            },
            PaymentMethodUpdate::ConnectorMandateDetailsUpdate {
                connector_mandate_details,
                last_modified_by,
            } => Self {
                payment_method_data: None,
                last_used_at: None,
                status: None,
                locker_id: None,
                payment_method: None,
                payment_method_type: None,
                payment_method_type_v2: None,
                connector_mandate_details,
                network_transaction_id: None,
                network_transaction_link_id: None,
                updated_by: None,
                payment_method_subtype: None,
                last_modified: now,
                network_token_locker_id: None,
                network_token_requestor_reference_id: None,
                network_token_payment_method_data: None,
                locker_fingerprint_id: None,
                external_vault_source: None,
                last_modified_by,
                customer_details: None,
                compatibility_updated_at: Some(now),
            },
            PaymentMethodUpdate::StatusAndFingerprintUpdate {
                status,
                locker_fingerprint_id,
                last_modified_by,
            } => Self {
                payment_method_data: None,
                last_used_at: None,
                network_transaction_id: None,
                network_transaction_link_id: None,
                status,
                locker_id: None,
                payment_method: None,
                payment_method_type: None,
                payment_method_type_v2: None,
                connector_mandate_details: None,
                updated_by: None,
                payment_method_subtype: None,
                last_modified: now,
                network_token_locker_id: None,
                network_token_requestor_reference_id: None,
                network_token_payment_method_data: None,
                locker_fingerprint_id,
                external_vault_source: None,
                last_modified_by,
                customer_details: None,
                compatibility_updated_at: Some(now),
            },
            PaymentMethodUpdate::PopulateLegacyCompatFields {
                payment_method,
                payment_method_type,
                connector_mandate_details,
                last_modified_by,
            } => Self {
                payment_method_data: None,
                last_used_at: None,
                network_transaction_id: None,
                network_transaction_link_id: None,
                status: None,
                locker_id: None,
                payment_method,
                payment_method_type,
                payment_method_type_v2: None,
                connector_mandate_details,
                updated_by: None,
                payment_method_subtype: None,
                last_modified: now,
                network_token_locker_id: None,
                network_token_requestor_reference_id: None,
                network_token_payment_method_data: None,
                locker_fingerprint_id: None,
                external_vault_source: None,
                last_modified_by,
                customer_details: None,
                compatibility_updated_at: Some(now),
            },
        }
    }
}

#[cfg(feature = "v1")]
impl From<&PaymentMethodNew> for PaymentMethod {
    fn from(payment_method_new: &PaymentMethodNew) -> Self {
        Self {
            customer_id: payment_method_new.customer_id.clone(),
            merchant_id: payment_method_new.merchant_id.clone(),
            payment_method_id: payment_method_new.payment_method_id.clone(),
            locker_id: payment_method_new.locker_id.clone(),
            network_token_requestor_reference_id: payment_method_new
                .network_token_requestor_reference_id
                .clone(),
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
            connector_payment_method_details: payment_method_new
                .connector_payment_method_details
                .clone(),
            connector_mandate_details: payment_method_new.connector_mandate_details.clone(),
            customer_acceptance: payment_method_new.customer_acceptance.clone(),
            status: payment_method_new.status,
            network_transaction_id: payment_method_new.network_transaction_id.clone(),
            network_transaction_link_id: payment_method_new.network_transaction_link_id.clone(),
            client_secret: payment_method_new.client_secret.clone(),
            updated_by: payment_method_new.updated_by.clone(),
            payment_method_billing_address: payment_method_new
                .payment_method_billing_address
                .clone(),
            version: payment_method_new.version,
            network_token_locker_id: payment_method_new.network_token_locker_id.clone(),
            network_token_payment_method_data: payment_method_new
                .network_token_payment_method_data
                .clone(),
            external_vault_source: payment_method_new.external_vault_source.clone(),
            vault_type: payment_method_new.vault_type,
            created_by: payment_method_new.created_by.clone(),
            last_modified_by: payment_method_new.last_modified_by.clone(),
            customer_details: payment_method_new.customer_details.clone(),
            locker_fingerprint_id: payment_method_new.locker_fingerprint_id.clone(),
            network_tokenization_data: payment_method_new.network_tokenization_data.clone(),
            payment_method_type_v2: None,
            payment_method_subtype: None,
            id: payment_method_new.id.clone(),
            compatibility_updated_at: payment_method_new.compatibility_updated_at,
            auxiliary_fingerprint_id: payment_method_new.auxiliary_fingerprint_id.clone(),
        }
    }
}

#[cfg(feature = "v2")]
impl From<&PaymentMethodNew> for PaymentMethod {
    fn from(payment_method_new: &PaymentMethodNew) -> Self {
        Self {
            customer_id: payment_method_new.customer_id.clone(),
            merchant_id: payment_method_new.merchant_id.clone(),
            locker_id: payment_method_new.locker_id.clone(),
            created_at: payment_method_new.created_at,
            last_modified: payment_method_new.last_modified,
            payment_method: payment_method_new.payment_method,
            payment_method_type: payment_method_new.payment_method_type,
            payment_method_data: payment_method_new.payment_method_data.clone(),
            last_used_at: payment_method_new.last_used_at,
            connector_mandate_details: payment_method_new.connector_mandate_details.clone(),
            customer_acceptance: payment_method_new.customer_acceptance.clone(),
            status: payment_method_new.status,
            network_transaction_id: payment_method_new
                .network_transaction_id
                .clone()
                .map(Secret::new),
            network_transaction_link_id: payment_method_new
                .network_transaction_link_id
                .clone()
                .map(Secret::new),
            client_secret: payment_method_new.client_secret.clone(),
            updated_by: payment_method_new.updated_by.clone(),
            payment_method_billing_address: payment_method_new
                .payment_method_billing_address
                .clone(),
            locker_fingerprint_id: payment_method_new.locker_fingerprint_id.clone(),
            auxiliary_fingerprint_id: payment_method_new.auxiliary_fingerprint_id.clone(),
            payment_method_type_v2: payment_method_new.payment_method_type_v2,
            payment_method_subtype: payment_method_new.payment_method_subtype,
            id: payment_method_new.id.clone(),
            payment_method_id: payment_method_new.payment_method_id.clone(),
            version: payment_method_new.version,
            network_token_requestor_reference_id: payment_method_new
                .network_token_requestor_reference_id
                .clone(),
            network_token_locker_id: payment_method_new.network_token_locker_id.clone(),
            network_token_payment_method_data: payment_method_new
                .network_token_payment_method_data
                .clone(),
            external_vault_token_data: payment_method_new.external_vault_token_data.clone(),
            vault_type: payment_method_new.vault_type,
            created_by: payment_method_new.created_by.clone(),
            last_modified_by: payment_method_new.last_modified_by.clone(),
            customer_details: payment_method_new.customer_details.clone(),
            network_tokenization_data: None,
            compatibility_updated_at: payment_method_new.compatibility_updated_at,
            external_vault_source: payment_method_new.external_vault_source.clone(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PaymentsMandateReferenceRecord {
    pub connector_mandate_id: String,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
    pub mandate_metadata: Option<pii::SecretSerdeValue>,
    pub connector_mandate_status: Option<common_enums::ConnectorMandateStatus>,
    pub connector_mandate_request_reference_id: Option<String>,
    pub connector_customer_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConnectorTokenReferenceRecord {
    pub connector_token: String,
    pub payment_method_subtype: Option<common_enums::PaymentMethodType>,
    pub original_payment_authorized_amount: Option<common_utils::types::MinorUnit>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_token_status: common_enums::ConnectorTokenStatus,
    pub connector_token_request_reference_id: Option<String>,
    pub connector_customer_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct PaymentsMandateReference(
    pub HashMap<common_utils::id_type::MerchantConnectorAccountId, PaymentsMandateReferenceRecord>,
);

impl std::ops::Deref for PaymentsMandateReference {
    type Target =
        HashMap<common_utils::id_type::MerchantConnectorAccountId, PaymentsMandateReferenceRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PaymentsMandateReference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct PaymentsTokenReference(
    pub HashMap<common_utils::id_type::MerchantConnectorAccountId, ConnectorTokenReferenceRecord>,
);

impl std::ops::Deref for PaymentsTokenReference {
    type Target =
        HashMap<common_utils::id_type::MerchantConnectorAccountId, ConnectorTokenReferenceRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PaymentsTokenReference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[cfg(feature = "v2")]
impl From<PaymentsMandateReference> for PaymentsTokenReference {
    fn from(payments_mandate_reference: PaymentsMandateReference) -> Self {
        let mapped_records = payments_mandate_reference
            .0
            .into_iter()
            .map(|(mca_id, record)| {
                let token_status = record
                    .connector_mandate_status
                    .map(common_enums::ConnectorTokenStatus::from)
                    .unwrap_or(common_enums::ConnectorTokenStatus::Inactive);

                let token_record = ConnectorTokenReferenceRecord {
                    connector_token: record.connector_mandate_id,
                    payment_method_subtype: record.payment_method_type,
                    original_payment_authorized_amount: record
                        .original_payment_authorized_amount
                        .map(common_utils::types::MinorUnit::new),
                    original_payment_authorized_currency: record
                        .original_payment_authorized_currency,
                    metadata: record.mandate_metadata,
                    connector_token_status: token_status,
                    connector_token_request_reference_id: record
                        .connector_mandate_request_reference_id,
                    connector_customer_id: record.connector_customer_id,
                };

                (mca_id, token_record)
            })
            .collect();

        Self(mapped_records)
    }
}

#[cfg(feature = "v1")]
common_utils::impl_to_sql_from_sql_json!(PaymentsMandateReference);

#[cfg(feature = "v2")]
common_utils::impl_to_sql_from_sql_json!(PaymentsTokenReference);

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PayoutsMandateReferenceRecord {
    pub transfer_method_id: Option<String>,
    pub connector_customer_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct PayoutsMandateReference(
    pub HashMap<common_utils::id_type::MerchantConnectorAccountId, PayoutsMandateReferenceRecord>,
);

impl std::ops::Deref for PayoutsMandateReference {
    type Target =
        HashMap<common_utils::id_type::MerchantConnectorAccountId, PayoutsMandateReferenceRecord>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PayoutsMandateReference {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ConnectorMandateCompatReference {
    pub connector_mandate_id: Option<String>,
    pub connector_token: Option<String>,
    pub payment_method_type: Option<common_enums::PaymentMethodType>,
    pub payment_method_subtype: Option<common_enums::PaymentMethodType>,
    pub connector_mandate_status: Option<common_enums::ConnectorMandateStatus>,
    pub connector_token_status: Option<common_enums::ConnectorTokenStatus>,
    pub connector_mandate_request_reference_id: Option<String>,
    pub connector_token_request_reference_id: Option<String>,
    pub original_payment_authorized_amount: Option<i64>,
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
    pub mandate_metadata: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_customer_id: Option<String>,
}

impl ConnectorMandateCompatReference {
    #[cfg(feature = "v1")]
    fn add_v2_fields(&mut self) {
        self.connector_token = self.connector_mandate_id.clone();
        self.payment_method_subtype = self.payment_method_type;
        self.connector_token_request_reference_id =
            self.connector_mandate_request_reference_id.clone();
        self.connector_token_status = self
            .connector_mandate_status
            .map(common_enums::ConnectorTokenStatus::from);
    }

    fn add_v1_fields(&mut self) {
        self.connector_mandate_id = self.connector_token.clone();
        self.payment_method_type = self.payment_method_subtype;
        self.connector_mandate_request_reference_id =
            self.connector_token_request_reference_id.clone();
        self.connector_mandate_status = self
            .connector_token_status
            .map(common_enums::ConnectorMandateStatus::from);
    }
}

#[derive(Clone, Debug)]
pub struct ConnectorMandateCompatDetails {
    payments:
        HashMap<common_utils::id_type::MerchantConnectorAccountId, ConnectorMandateCompatReference>,
    payouts: Option<PayoutsMandateReference>,
}

impl TryFrom<serde_json::Value> for ConnectorMandateCompatDetails {
    type Error = ParsingError;

    fn try_from(mut connector_mandate_details: serde_json::Value) -> Result<Self, Self::Error> {
        let payment_connector_references =
            connector_mandate_details
                .as_object_mut()
                .ok_or(ParsingError::StructParseFailure(
                    "connector mandate details",
                ))?;
        let payouts = payment_connector_references
            .remove("payouts")
            .map(serde_json::from_value)
            .transpose()
            .map_err(|_| ParsingError::StructParseFailure("payout mandate details"))?;
        let payments = serde_json::from_value(serde_json::Value::Object(std::mem::take(
            payment_connector_references,
        )))
        .map_err(|_| ParsingError::StructParseFailure("payment mandate details"))?;

        Ok(Self { payments, payouts })
    }
}

impl ConnectorMandateCompatDetails {
    pub fn into_value(self) -> Option<serde_json::Value> {
        let mut connector_mandate_details_value = serde_json::to_value(self.payments).ok()?;

        if let Some(payouts) = self.payouts {
            let payouts = serde_json::to_value(payouts).ok()?;
            connector_mandate_details_value.as_object_mut().map(
                |payment_connector_references| {
                    payment_connector_references.insert("payouts".to_string(), payouts);
                },
            )?;
        }

        Some(connector_mandate_details_value)
    }

    #[cfg(feature = "v1")]
    fn add_v2_fields(mut self) -> Self {
        self.payments
            .values_mut()
            .for_each(ConnectorMandateCompatReference::add_v2_fields);
        self
    }

    fn add_v1_fields(mut self) -> Self {
        self.payments
            .values_mut()
            .for_each(ConnectorMandateCompatReference::add_v1_fields);
        self
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct CommonMandateReference {
    pub payments: Option<PaymentsMandateReference>,
    pub payouts: Option<PayoutsMandateReference>,
}

#[cfg(feature = "v2")]
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize, diesel::AsExpression)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct CommonMandateReference {
    pub payments: Option<PaymentsTokenReference>,
    pub payouts: Option<PayoutsMandateReference>,
}

impl CommonMandateReference {
    pub fn get_mandate_details_value(&self) -> CustomResult<serde_json::Value, ParsingError> {
        let mut payments = self
            .payments
            .as_ref()
            .map_or_else(|| Ok(serde_json::json!({})), serde_json::to_value)
            .change_context(ParsingError::StructParseFailure("payment mandate details"))?;

        self.payouts
            .as_ref()
            .map(|payouts_mandate| {
                serde_json::to_value(payouts_mandate).map(|payouts_mandate_value| {
                    payments.as_object_mut().map(|payments_object| {
                        payments_object.insert("payouts".to_string(), payouts_mandate_value);
                    })
                })
            })
            .transpose()
            .change_context(ParsingError::StructParseFailure("payout mandate details"))?;

        #[cfg(feature = "v2")]
        {
            if let Some(updated_payments) =
                Self::parse_connector_mandate_compat_details(Some(payments.clone()))
                    .map(Self::add_v1_connector_mandate_fields)
                    .and_then(ConnectorMandateCompatDetails::into_value)
            {
                payments = updated_payments;
            }
        }

        Ok(payments)
    }

    #[cfg(feature = "v1")]
    pub fn parse_payments_reference_with_token_fallback(
        payments_json: serde_json::Value,
    ) -> CustomResult<PaymentsMandateReference, ParsingError> {
        match serde_json::from_value::<PaymentsMandateReference>(payments_json.clone()) {
            Ok(mandate_reference) => Ok(mandate_reference),
            Err(mandate_err) => {
                router_env::logger::warn!(
                    "Failed to parse connector_mandate_details as PaymentsMandateReference: {}. Falling back to PaymentsTokenReference parser",
                    mandate_err
                );

                let token_reference =
                    serde_json::from_value::<PaymentsTokenReference>(payments_json)
                        .inspect_err(|token_err| {
                            router_env::logger::error!(
                        "Failed to parse connector_mandate_details as PaymentsTokenReference: {}",
                        token_err
                    );
                        })
                        .change_context(ParsingError::StructParseFailure(
                            "Failed to parse payments data",
                        ))?;

                let mandate_reference = PaymentsMandateReference(
                    token_reference
                        .0
                        .into_iter()
                        .map(|(mca_id, token_record)| {
                            let connector_mandate_status =
                                token_record.connector_token_status.into();

                            (
                                mca_id,
                                PaymentsMandateReferenceRecord {
                                    connector_mandate_id: token_record.connector_token,
                                    payment_method_type: token_record.payment_method_subtype,
                                    original_payment_authorized_amount: token_record
                                        .original_payment_authorized_amount
                                        .map(|amount| amount.get_amount_as_i64()),
                                    original_payment_authorized_currency: token_record
                                        .original_payment_authorized_currency,
                                    mandate_metadata: token_record.metadata,
                                    connector_mandate_status: Some(connector_mandate_status),
                                    connector_mandate_request_reference_id: token_record
                                        .connector_token_request_reference_id,
                                    connector_customer_id: token_record.connector_customer_id,
                                },
                            )
                        })
                        .collect(),
                );

                router_env::logger::info!(
                    "Parsed connector_mandate_details using token shape fallback"
                );

                Ok(mandate_reference)
            }
        }
    }

    #[cfg(feature = "v2")]
    fn parse_payments_reference_with_legacy_fallback(
        payments_json: serde_json::Value,
    ) -> CustomResult<PaymentsTokenReference, ParsingError> {
        match serde_json::from_value::<PaymentsTokenReference>(payments_json.clone()) {
            Ok(token_reference) => Ok(token_reference),
            Err(token_err) => {
                router_env::logger::warn!(
                    "Failed to parse connector_mandate_details as PaymentsTokenReference: {}. Falling back to PaymentsMandateReference parser",
                    token_err
                );

                let legacy_reference =
                    serde_json::from_value::<PaymentsMandateReference>(payments_json)
                        .inspect_err(|legacy_err| {
                            router_env::logger::error!(
                        "Failed to parse connector_mandate_details as PaymentsMandateReference: {}",
                        legacy_err
                    );
                        })
                        .change_context(ParsingError::StructParseFailure(
                            "Failed to parse payments data",
                        ))?;

                router_env::logger::info!(
                    "Parsed connector_mandate_details using legacy mandate shape fallback"
                );

                Ok(PaymentsTokenReference::from(legacy_reference))
            }
        }
    }

    /// Add V2-compatible connector mandate keys into an existing V1 connector mandate JSON.
    ///
    /// This keeps old keys in place and only augments each payment connector entry with
    /// V2-specific keys when missing.
    #[cfg(feature = "v1")]
    pub fn add_v2_connector_mandate_fields(
        connector_mandate_details: ConnectorMandateCompatDetails,
    ) -> ConnectorMandateCompatDetails {
        connector_mandate_details.add_v2_fields()
    }

    pub fn parse_connector_mandate_compat_details(
        connector_mandate_details: Option<serde_json::Value>,
    ) -> Option<ConnectorMandateCompatDetails> {
        ConnectorMandateCompatDetails::try_from(connector_mandate_details?).ok()
    }

    /// Add V1-compatible connector mandate keys into an existing V2 connector mandate JSON.
    ///
    /// This keeps new keys in place and only augments each payment connector entry with
    /// V1-specific keys when missing.
    pub fn add_v1_connector_mandate_fields(
        connector_mandate_details: ConnectorMandateCompatDetails,
    ) -> ConnectorMandateCompatDetails {
        connector_mandate_details.add_v1_fields()
    }

    #[cfg(feature = "v2")]
    /// Insert a new payment token reference for the given connector_id
    pub fn insert_payment_token_reference_record(
        &mut self,
        connector_id: &common_utils::id_type::MerchantConnectorAccountId,
        record: ConnectorTokenReferenceRecord,
    ) {
        match self.payments {
            Some(ref mut payments_reference) => {
                payments_reference.insert(connector_id.clone(), record);
            }
            None => {
                let mut payments_reference = HashMap::new();
                payments_reference.insert(connector_id.clone(), record);
                self.payments = Some(PaymentsTokenReference(payments_reference));
            }
        }
    }
}

impl diesel::serialize::ToSql<diesel::sql_types::Jsonb, diesel::pg::Pg> for CommonMandateReference {
    fn to_sql<'b>(
        &'b self,
        out: &mut diesel::serialize::Output<'b, '_, diesel::pg::Pg>,
    ) -> diesel::serialize::Result {
        let payments = self.get_mandate_details_value()?;

        <serde_json::Value as diesel::serialize::ToSql<
            diesel::sql_types::Jsonb,
            diesel::pg::Pg,
        >>::to_sql(&payments, &mut out.reborrow())
    }
}

#[cfg(feature = "v1")]
impl<DB: diesel::backend::Backend> diesel::deserialize::FromSql<diesel::sql_types::Jsonb, DB>
    for CommonMandateReference
where
    serde_json::Value: diesel::deserialize::FromSql<diesel::sql_types::Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let value = <serde_json::Value as diesel::deserialize::FromSql<
            diesel::sql_types::Jsonb,
            DB,
        >>::from_sql(bytes)?;

        let payments_data = value
            .clone()
            .as_object_mut()
            .map(|obj| {
                obj.remove("payouts");

                Self::parse_payments_reference_with_token_fallback(serde_json::Value::Object(
                    obj.clone(),
                ))
            })
            .transpose()?;

        let payouts_data = serde_json::from_value::<Option<Self>>(value)
            .inspect_err(|err| {
                router_env::logger::error!("Failed to parse payouts data: {}", err);
            })
            .change_context(ParsingError::StructParseFailure(
                "Failed to parse payouts data",
            ))
            .map(|optional_common_mandate_details| {
                optional_common_mandate_details
                    .and_then(|common_mandate_details| common_mandate_details.payouts)
            })?;

        Ok(Self {
            payments: payments_data,
            payouts: payouts_data,
        })
    }
}

#[cfg(feature = "v2")]
impl<DB: diesel::backend::Backend> diesel::deserialize::FromSql<diesel::sql_types::Jsonb, DB>
    for CommonMandateReference
where
    serde_json::Value: diesel::deserialize::FromSql<diesel::sql_types::Jsonb, DB>,
{
    fn from_sql(bytes: DB::RawValue<'_>) -> diesel::deserialize::Result<Self> {
        let value = <serde_json::Value as diesel::deserialize::FromSql<
            diesel::sql_types::Jsonb,
            DB,
        >>::from_sql(bytes)?;

        let payments_data = value
            .clone()
            .as_object_mut()
            .map(|obj| {
                obj.remove("payouts");

                Self::parse_payments_reference_with_legacy_fallback(serde_json::Value::Object(
                    obj.clone(),
                ))
            })
            .transpose()?;

        let payouts_data = serde_json::from_value::<Option<Self>>(value)
            .inspect_err(|err| {
                router_env::logger::error!("Failed to parse payouts data: {}", err);
            })
            .change_context(ParsingError::StructParseFailure(
                "Failed to parse payouts data",
            ))
            .map(|optional_common_mandate_details| {
                optional_common_mandate_details
                    .and_then(|common_mandate_details| common_mandate_details.payouts)
            })?;

        Ok(Self {
            payments: payments_data,
            payouts: payouts_data,
        })
    }
}

#[cfg(feature = "v1")]
impl From<PaymentsMandateReference> for CommonMandateReference {
    fn from(payment_reference: PaymentsMandateReference) -> Self {
        Self {
            payments: Some(payment_reference),
            payouts: None,
        }
    }
}
