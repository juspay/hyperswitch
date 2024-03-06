use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};
use serde_json;

use crate::schema::authentication;

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = authentication,  primary_key(authentication_id))]
pub struct Authentication {
    pub authentication_id: String,
    pub merchant_id: String,
    pub authentication_connector: String,
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: common_enums::AuthenticationStatus,
    pub authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: time::PrimitiveDateTime,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Insertable)]
#[diesel(table_name = authentication)]
pub struct AuthenticationNew {
    pub authentication_id: String,
    pub merchant_id: String,
    pub authentication_connector: String,
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: common_enums::AuthenticationStatus,
    pub authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum AuthenticationUpdate {
    AuthenticationDataUpdate {
        authentication_data: Option<serde_json::Value>,
        connector_authentication_id: Option<String>,
        payment_method_id: Option<String>,
        authentication_type: Option<common_enums::DecoupledAuthenticationType>,
        authentication_status: Option<common_enums::AuthenticationStatus>,
        authentication_lifecycle_status: Option<common_enums::AuthenticationLifecycleStatus>,
        connector_metadata: Option<serde_json::Value>,
    },
    PostAuthorizationUpdate {
        authentication_lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    },
    ErrorUpdate {
        error_message: Option<String>,
        error_code: Option<String>,
        authentication_status: common_enums::AuthenticationStatus,
        connector_authentication_id: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, AsChangeset, Serialize, Deserialize)]
#[diesel(table_name = authentication)]
pub struct AuthenticationUpdateInternal {
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: Option<String>,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: Option<common_enums::AuthenticationStatus>,
    pub authentication_lifecycle_status: Option<common_enums::AuthenticationLifecycleStatus>,
    pub modified_at: time::PrimitiveDateTime,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub connector_metadata: Option<serde_json::Value>,
}

impl AuthenticationUpdateInternal {
    pub fn apply_changeset(self, source: Authentication) -> Authentication {
        let Self {
            connector_authentication_id,
            authentication_data,
            payment_method_id,
            authentication_type,
            authentication_status,
            authentication_lifecycle_status,
            modified_at: _,
            error_code,
            error_message,
            connector_metadata,
        } = self;
        Authentication {
            connector_authentication_id: connector_authentication_id
                .or(source.connector_authentication_id),
            authentication_data: authentication_data.or(source.authentication_data),
            payment_method_id: payment_method_id.unwrap_or(source.payment_method_id),
            authentication_type: authentication_type.or(source.authentication_type),
            authentication_status: authentication_status.unwrap_or(source.authentication_status),
            authentication_lifecycle_status: authentication_lifecycle_status
                .unwrap_or(source.authentication_lifecycle_status),
            modified_at: common_utils::date_time::now(),
            error_code: error_code.or(source.error_code),
            error_message: error_message.or(source.error_message),
            connector_metadata: connector_metadata.or(source.connector_metadata),
            ..source
        }
    }
}

impl From<AuthenticationUpdate> for AuthenticationUpdateInternal {
    fn from(auth_update: AuthenticationUpdate) -> Self {
        match auth_update {
            AuthenticationUpdate::AuthenticationDataUpdate {
                authentication_data,
                connector_authentication_id,
                authentication_type,
                authentication_status,
                payment_method_id,
                authentication_lifecycle_status,
                connector_metadata,
            } => Self {
                authentication_data,
                connector_authentication_id,
                authentication_type,
                authentication_status,
                authentication_lifecycle_status,
                modified_at: common_utils::date_time::now(),
                payment_method_id,
                error_message: None,
                error_code: None,
                connector_metadata,
            },
            AuthenticationUpdate::ErrorUpdate {
                error_message,
                error_code,
                authentication_status,
                connector_authentication_id,
            } => Self {
                error_code,
                error_message,
                authentication_status: Some(authentication_status),
                authentication_data: None,
                connector_authentication_id,
                authentication_type: None,
                authentication_lifecycle_status: None,
                modified_at: common_utils::date_time::now(),
                payment_method_id: None,
                connector_metadata: None,
            },
            AuthenticationUpdate::PostAuthorizationUpdate {
                authentication_lifecycle_status,
            } => Self {
                connector_authentication_id: None,
                authentication_data: None,
                payment_method_id: None,
                authentication_type: None,
                authentication_status: None,
                authentication_lifecycle_status: Some(authentication_lifecycle_status),
                modified_at: common_utils::date_time::now(),
                error_message: None,
                error_code: None,
                connector_metadata: None,
            },
        }
    }
}
