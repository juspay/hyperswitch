use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};
use serde_json;

use crate::schema::authentication;

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = authentication,  primary_key(authentication_id))]
pub struct Authentication {
    pub authentication_id: String,
    pub merchant_id: String,
    pub connector: String,
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: common_enums::AuthenticationStatus,
    pub lifecycle_status: common_enums::AuthenticationLifecycleStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: time::PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, Eq, PartialEq, Queryable, Serialize, Deserialize, Insertable)]
#[diesel(table_name = authentication)]
pub struct AuthenticationNew {
    pub authentication_id: String,
    pub merchant_id: String,
    pub connector: String,
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: common_enums::AuthenticationStatus,
    pub lifecycle_status: common_enums::AuthenticationLifecycleStatus,
}

#[derive(Debug)]
pub enum AuthenticationUpdate {
    AuthenticationDataUpdate {
        authentication_data: Option<serde_json::Value>,
        connector_authentication_id: Option<String>,
        payment_method_id: Option<String>,
        authentication_type: Option<common_enums::DecoupledAuthenticationType>,
        authentication_status: Option<common_enums::AuthenticationStatus>,
        lifecycle_status: Option<common_enums::AuthenticationLifecycleStatus>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, AsChangeset, Queryable, Serialize, Deserialize)]
#[diesel(table_name = authentication)]
pub struct AuthenticationUpdateInternal {
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: Option<String>,
    pub authentication_type: Option<common_enums::DecoupledAuthenticationType>,
    pub authentication_status: Option<common_enums::AuthenticationStatus>,
    pub lifecycle_status: Option<common_enums::AuthenticationLifecycleStatus>,
    pub modified_at: time::PrimitiveDateTime,
}

impl AuthenticationUpdateInternal {
    pub fn apply_changeset(self, source: Authentication) -> Authentication {
        let Self {
            connector_authentication_id,
            authentication_data,
            payment_method_id,
            authentication_type,
            authentication_status,
            lifecycle_status,
            modified_at: _,
        } = self;
        Authentication {
            connector_authentication_id: connector_authentication_id
                .or(source.connector_authentication_id),
            authentication_data: authentication_data.or(source.authentication_data),
            payment_method_id: payment_method_id.unwrap_or(source.payment_method_id),
            authentication_type: authentication_type.or(source.authentication_type),
            authentication_status: authentication_status.unwrap_or(source.authentication_status),
            lifecycle_status: lifecycle_status.unwrap_or(source.lifecycle_status),
            modified_at: common_utils::date_time::now(),
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
                lifecycle_status,
            } => Self {
                authentication_data,
                connector_authentication_id,
                authentication_type,
                authentication_status,
                lifecycle_status,
                modified_at: common_utils::date_time::now(),
                payment_method_id,
            },
        }
    }
}
