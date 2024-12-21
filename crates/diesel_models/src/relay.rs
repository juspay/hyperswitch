use crate::{enums as storage_enums, schema::relay};
use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

#[derive(
    Clone,
    Debug,
    Eq,
    Identifiable,
    Queryable,
    Selectable,
    PartialEq,
    serde::Serialize,
    serde::Deserialize,
)]
#[diesel(table_name = relay)]
pub struct Relay {
    pub id: common_utils::id_type::RelayId,
    pub connector_resource_id: String,
    pub connector_id: common_utils::id_type::MerchantConnectorAccountId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub relay_type: storage_enums::RelayType,
    pub request_data: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::RelayStatus,
    pub connector_reference_id: Option<String>,
    pub error_code: Option<String>,
    pub error_reason: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub response_data: Option<pii::SecretSerdeValue>,
}

#[derive(
    Clone,
    Debug,
    Eq,
    PartialEq,
    Insertable,
    router_derive::DebugAsDisplay,
    serde::Serialize,
    serde::Deserialize,
    router_derive::Setter,
)]
#[diesel(table_name = relay)]
pub struct RelayNew {
    pub id: common_utils::id_type::RelayId,
    pub connector_resource_id: String,
    pub connector_id: common_utils::id_type::MerchantConnectorAccountId,
    pub profile_id: common_utils::id_type::ProfileId,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub relay_type: storage_enums::RelayType,
    pub request_data: Option<pii::SecretSerdeValue>,
    pub status: storage_enums::RelayStatus,
    pub connector_reference_id: Option<String>,
    pub error_code: Option<String>,
    pub error_reason: Option<String>,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub response_data: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum RelayUpdate {
    ErrorUpdate {
        error_code: String,
        error_reason: String,
    },
    StatusUpdate {
        connector_reference_id: Option<String>,
        status: storage_enums::RelayStatus,
    },
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[table_name = "relay"]
pub struct RelayUpdateInternal {
    pub connector_reference_id: Option<String>,
    pub status: Option<storage_enums::RelayStatus>,
    pub error_code: Option<String>,
    pub error_reason: Option<String>,
    pub modified_at: PrimitiveDateTime,
}

impl From<RelayUpdate> for RelayUpdateInternal {
    fn from(value: RelayUpdate) -> Self {
        match value {
            RelayUpdate::ErrorUpdate {
                error_code,
                error_reason,
            } => Self {
                error_code: Some(error_code),
                error_reason: Some(error_reason),
                connector_reference_id: None,
                status: None,
                modified_at: common_utils::date_time::now(),
            },
            RelayUpdate::StatusUpdate {
                connector_reference_id,
                status,
            } => Self {
                connector_reference_id,
                status: Some(status),
                error_code: None,
                error_reason: None,
                modified_at: common_utils::date_time::now(),
            },
        }
    }
}
