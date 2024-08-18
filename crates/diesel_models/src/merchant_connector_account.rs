use std::fmt::Debug;

use common_utils::{encryption::Encryption, id_type, pii};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::enums as storage_enums;
#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_connector_account_v2")
))]
use crate::schema::merchant_connector_account;
#[cfg(all(feature = "v2", feature = "merchant_connector_account_v2"))]
use crate::schema_v2::merchant_connector_account;

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_connector_account_v2")
))]
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
    pub merchant_connector_id: String,
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
    pub profile_id: Option<String>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::ConnectorStatus,
    pub additional_merchant_data: Option<Encryption>,
    pub connector_wallets_details: Option<Encryption>,
    pub version: common_enums::ApiVersion,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_connector_account_v2")
))]
impl MerchantConnectorAccount {
    pub fn get_id(&self) -> String {
        self.merchant_connector_id.clone()
    }
}

#[cfg(all(feature = "v2", feature = "merchant_connector_account_v2"))]
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
    pub connector_name: String,
    pub connector_account_details: Encryption,
    pub disabled: Option<bool>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub payment_methods_enabled: Option<Vec<pii::SecretSerdeValue>>,
    pub connector_type: storage_enums::ConnectorType,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_label: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub connector_webhook_details: Option<pii::SecretSerdeValue>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub frm_config: Option<Vec<pii::SecretSerdeValue>>,
    pub profile_id: String,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::ConnectorStatus,
    pub additional_merchant_data: Option<Encryption>,
    pub connector_wallets_details: Option<Encryption>,
    pub id: String,
    pub version: common_enums::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "merchant_connector_account_v2"))]
impl MerchantConnectorAccount {
    pub fn get_id(&self) -> String {
        self.id.clone()
    }
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_connector_account_v2")
))]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccountNew {
    pub merchant_id: Option<id_type::MerchantId>,
    pub connector_type: Option<storage_enums::ConnectorType>,
    pub connector_name: Option<String>,
    pub connector_account_details: Option<Encryption>,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: String,
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
    pub profile_id: Option<String>,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::ConnectorStatus,
    pub additional_merchant_data: Option<Encryption>,
    pub connector_wallets_details: Option<Encryption>,
    pub version: common_enums::ApiVersion,
}

#[cfg(all(feature = "v2", feature = "merchant_connector_account_v2"))]
#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccountNew {
    pub merchant_id: Option<id_type::MerchantId>,
    pub connector_type: Option<storage_enums::ConnectorType>,
    pub connector_name: Option<String>,
    pub connector_account_details: Option<Encryption>,
    pub disabled: Option<bool>,
    pub payment_methods_enabled: Option<Vec<pii::SecretSerdeValue>>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub connector_label: Option<String>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
    pub connector_webhook_details: Option<pii::SecretSerdeValue>,
    #[diesel(deserialize_as = super::OptionalDieselArray<pii::SecretSerdeValue>)]
    pub frm_config: Option<Vec<pii::SecretSerdeValue>>,
    pub profile_id: String,
    #[diesel(deserialize_as = super::OptionalDieselArray<String>)]
    pub applepay_verified_domains: Option<Vec<String>>,
    pub pm_auth_config: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::ConnectorStatus,
    pub additional_merchant_data: Option<Encryption>,
    pub connector_wallets_details: Option<Encryption>,
    pub id: String,
    pub version: common_enums::ApiVersion,
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_connector_account_v2")
))]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccountUpdateInternal {
    pub connector_type: Option<storage_enums::ConnectorType>,
    pub connector_name: Option<String>,
    pub connector_account_details: Option<Encryption>,
    pub connector_label: Option<String>,
    pub test_mode: Option<bool>,
    pub disabled: Option<bool>,
    pub merchant_connector_id: Option<String>,
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
}

#[cfg(all(feature = "v2", feature = "merchant_connector_account_v2"))]
#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = merchant_connector_account)]
pub struct MerchantConnectorAccountUpdateInternal {
    pub connector_type: Option<storage_enums::ConnectorType>,
    pub connector_account_details: Option<Encryption>,
    pub connector_label: Option<String>,
    pub disabled: Option<bool>,
    pub payment_methods_enabled: Option<Vec<pii::SecretSerdeValue>>,
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
}

#[cfg(all(
    any(feature = "v1", feature = "v2"),
    not(feature = "merchant_connector_account_v2")
))]
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

#[cfg(all(feature = "v2", feature = "merchant_connector_account_v2"))]
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

            ..source
        }
    }
}
