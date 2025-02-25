#[cfg(feature = "v2")]
use std::collections::HashMap;
use std::fmt::Debug;

use common_utils::{encryption::Encryption, id_type, pii};
#[cfg(feature = "v2")]
use diesel::{sql_types::Jsonb, AsExpression};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::enums as storage_enums;
#[cfg(feature = "v1")]
use crate::schema::merchant_connector_account;
#[cfg(feature = "v2")]
use crate::{enums, schema_v2::merchant_connector_account, types};

#[cfg(feature = "v1")]
#[derive(
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Identifiable,
    Queryable,
    Selectable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchant_connector_account, primary_key(merchant_connector_id), check_for_backend(diesel::pg::Pg))]
pub struct MerchantConnectorAccount {
    pub merchant_id: id_type::MerchantId,
    pub connector_name: String,
    pub connector_account_details: Encryption,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub payment_methods_enabled: Option<Vec<pii::SecretSerdeValue>>,
    pub connector_type: storage_enums::ConnectorType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_label: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub business_sub_label: Option<String>,
    pub frm_configs: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub connector_webhook_details: Option<pii::SecretSerdeValue>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub frm_config: Option<Vec<pii::SecretSerdeValue>>,
    pub profile_id: Option<id_type::ProfileId>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::ConnectorStatus,
    pub additional_merchant_data: Option<Encryption>,
    pub connector_wallets_details: Option<Encryption>,
    pub version: common_enums::ApiVersion,
}

#[cfg(feature = "v1")]
impl MerchantConnectorAccount {
    pub fn get_id(&self) -> id_type::MerchantConnectorAccountId {
        self.merchant_connector_id.clone()
    }
}

#[cfg(feature = "v2")]
#[derive(
    Clone,
    Debug,
    serde::Serialize,
    serde::Deserialize,
    Identifiable,
    Queryable,
    Selectable,
    router_derive::DebugAsDisplay,
)]
#[diesel(table_name = merchant_connector_account, check_for_backend(diesel::pg::Pg))]
pub struct MerchantConnectorAccount {
    pub merchant_id: id_type::MerchantId,
    pub connector_name: common_enums::connector_enums::Connector,
    pub connector_account_details: Encryption,
    pub disabled: Option<bool>,
    #[diesel(deserialize_as = super::OptionalDieselArray<common_types::payment_methods::PaymentMethodsEnabled>)]
    pub payment_methods_enabled: Option<Vec<common_types::payment_methods::PaymentMethodsEnabled>>,
    pub connector_type: storage_enums::ConnectorType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_label: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub connector_webhook_details: Option<pii::SecretSerdeValue>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub frm_config: Option<Vec<pii::SecretSerdeValue>>,
    pub profile_id: id_type::ProfileId,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::ConnectorStatus,
    pub additional_merchant_data: Option<Encryption>,
    pub connector_wallets_details: Option<Encryption>,
    pub version: common_enums::ApiVersion,
    pub id: id_type::MerchantConnectorAccountId,
    pub feature_metadata: Option<MerchantConnectorAccountFeatureMetadata>,
}

#[cfg(feature = "v2")]
impl MerchantConnectorAccount {
    pub fn get_id(&self) -> id_type::MerchantConnectorAccountId {
        self.id.clone()
    }
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccountNew {
    pub merchant_id: Option<id_type::MerchantId>,
    pub connector_type: Option<storage_enums::ConnectorType>,
    pub connector_name: Option<String>,
    pub connector_account_details: Option<Encryption>,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: id_type::MerchantConnectorAccountId,
    pub payment_methods_enabled: Option<Vec<pii::SecretSerdeValue>>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_label: Option<String>,
    pub business_country: Option<storage_enums::CountryAlpha2>,
    pub business_label: Option<String>,
    pub business_sub_label: Option<String>,
    pub frm_configs: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub connector_webhook_details: Option<pii::SecretSerdeValue>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub frm_config: Option<Vec<pii::SecretSerdeValue>>,
    pub profile_id: Option<id_type::ProfileId>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::ConnectorStatus,
    pub additional_merchant_data: Option<Encryption>,
    pub connector_wallets_details: Option<Encryption>,
    pub version: common_enums::ApiVersion,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccountNew {
    pub merchant_id: Option<id_type::MerchantId>,
    pub connector_type: Option<storage_enums::ConnectorType>,
    pub connector_name: Option<common_enums::connector_enums::Connector>,
    pub connector_account_details: Option<Encryption>,
    pub disabled: Option<bool>,
    #[diesel(deserialize_as = super::OptionalDieselArray<common_types::payment_methods::PaymentMethodsEnabled>)]
    pub payment_methods_enabled: Option<Vec<common_types::payment_methods::PaymentMethodsEnabled>>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_label: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub connector_webhook_details: Option<pii::SecretSerdeValue>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub frm_config: Option<Vec<pii::SecretSerdeValue>>,
    pub profile_id: id_type::ProfileId,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::ConnectorStatus,
    pub additional_merchant_data: Option<Encryption>,
    pub connector_wallets_details: Option<Encryption>,
    pub id: id_type::MerchantConnectorAccountId,
    pub version: common_enums::ApiVersion,
    pub feature_metadata: Option<MerchantConnectorAccountFeatureMetadata>,
}

#[cfg(feature = "v1")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccountUpdateInternal {
    pub connector_type: Option<storage_enums::ConnectorType>,
    pub connector_name: Option<String>,
    pub connector_account_details: Option<Encryption>,
    pub connector_label: Option<String>,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: Option<id_type::MerchantConnectorAccountId>,
    pub payment_methods_enabled: Option<Vec<pii::SecretSerdeValue>>,
    pub frm_configs: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: Option<time::PrimitiveDateTime>,
    pub connector_webhook_details: Option<pii::SecretSerdeValue>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub frm_config: Option<Vec<pii::SecretSerdeValue>>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: Option<storage_enums::ConnectorStatus>,
    pub connector_wallets_details: Option<Encryption>,
    pub additional_merchant_data: Option<Encryption>,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccountUpdateInternal {
    pub connector_type: Option<storage_enums::ConnectorType>,
    pub connector_account_details: Option<Encryption>,
    pub connector_label: Option<String>,
    pub disabled: Option<bool>,
    #[diesel(deserialize_as = super::OptionalDieselArray<common_types::payment_methods::PaymentMethodsEnabled>)]
    pub payment_methods_enabled: Option<Vec<common_types::payment_methods::PaymentMethodsEnabled>>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: Option<time::PrimitiveDateTime>,
    pub connector_webhook_details: Option<pii::SecretSerdeValue>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub frm_config: Option<Vec<pii::SecretSerdeValue>>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: Option<storage_enums::ConnectorStatus>,
    pub connector_wallets_details: Option<Encryption>,
    pub additional_merchant_data: Option<Encryption>,
    pub feature_metadata: Option<MerchantConnectorAccountFeatureMetadata>,
}

#[cfg(feature = "v1")]
impl MerchantConnectorAccountUpdateInternal {
    pub fn create_merchant_connector_account(
        self,
        source: MerchantConnectorAccount,
    ) -> MerchantConnectorAccount {
        MerchantConnectorAccount {
            merchant_id: source.merchant_id,
            connector_type: self.connector_type.unwrap_or(source.connector_type),
            connector_account_details: self
                .connector_account_details
                .unwrap_or(source.connector_account_details),
            test_mode: self.test_mode,
            disabled: self.disabled,
            merchant_connector_id: self
                .merchant_connector_id
                .unwrap_or(source.merchant_connector_id),
            payment_methods_enabled: self.payment_methods_enabled,
            frm_config: self.frm_config,
            modified_at: self.modified_at.unwrap_or(source.modified_at),
            pm_auth_config: self.pm_auth_config,
            status: self.status.unwrap_or(source.status),

            ..source
        }
    }
}

#[cfg(feature = "v2")]
impl MerchantConnectorAccountUpdateInternal {
    pub fn create_merchant_connector_account(
        self,
        source: MerchantConnectorAccount,
    ) -> MerchantConnectorAccount {
        MerchantConnectorAccount {
            connector_type: self.connector_type.unwrap_or(source.connector_type),
            connector_account_details: self
                .connector_account_details
                .unwrap_or(source.connector_account_details),
            disabled: self.disabled,
            payment_methods_enabled: self.payment_methods_enabled,
            frm_config: self.frm_config,
            modified_at: self.modified_at.unwrap_or(source.modified_at),
            pm_auth_config: self.pm_auth_config,
            status: self.status.unwrap_or(source.status),
            feature_metadata: self.feature_metadata,

            ..source
        }
    }
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, AsExpression)]
#[diesel(sql_type = Jsonb)]
pub struct MerchantConnectorAccountFeatureMetadata {
    pub revenue_recovery: Option<RevenueRecoveryMetadata>,
}

#[cfg(feature = "v2")]
common_utils::impl_to_sql_from_sql_json!(MerchantConnectorAccountFeatureMetadata);

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RevenueRecoveryMetadata {
    /// The maximum number of retries allowed for an invoice. This limit is set by the merchant for each `billing connector`.Once this limit is reached, no further retries will be attempted.
    pub max_retry_count: u16,
    /// Maximum number of `billing connector` retries before revenue recovery can start executing retries.
    pub billing_connector_retry_threshold: u16,
    /// Billing account reference id is payment gateway id at billing connector end.
    /// Merchants need to provide a mapping between these merchant connector account and the corresponding  
    /// account reference IDs for each `billing connector`.
    pub billing_account_reference: BillingAccountReference,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BillingAccountReference(pub HashMap<id_type::MerchantConnectorAccountId, String>);
