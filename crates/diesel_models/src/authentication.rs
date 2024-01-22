use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{self, Deserialize, Serialize};
use serde_json;

use crate::{enums as storage_enums, schema::authentication};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize)]
#[diesel(table_name = authentication,  primary_key(authentication_id))]
pub struct Authentication {
    pub authentication_id: String,
    pub merchant_id: String,
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<storage_enums::DecoupledAuthenticationType>,
    pub authentication_status: storage_enums::AuthenticationStatus,
    pub lifecycle_status: storage_enums::AuthenticationLifecycleStatus,
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
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: String,
    pub authentication_type: Option<storage_enums::DecoupledAuthenticationType>,
    pub authentication_status: storage_enums::AuthenticationStatus,
    pub lifecycle_status: storage_enums::AuthenticationLifecycleStatus,
}

#[derive(Debug)]
pub enum AuthenticationUpdate {
    AuthenticationDataUpdate {
        authentication_data: Option<serde_json::Value>,
        connector_authentication_id: Option<String>,
        payment_method_id: Option<String>,
        authentication_type: Option<storage_enums::DecoupledAuthenticationType>,
        authentication_status: Option<storage_enums::AuthenticationStatus>,
        lifecycle_status: Option<storage_enums::AuthenticationLifecycleStatus>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, AsChangeset, Queryable, Serialize, Deserialize)]
#[diesel(table_name = authentication)]
pub struct AuthenticationUpdateInternal {
    pub connector_authentication_id: Option<String>,
    pub authentication_data: Option<serde_json::Value>,
    pub payment_method_id: Option<String>,
    pub authentication_type: Option<storage_enums::DecoupledAuthenticationType>,
    pub authentication_status: Option<storage_enums::AuthenticationStatus>,
    pub lifecycle_status: Option<storage_enums::AuthenticationLifecycleStatus>,
    pub modified_at: time::PrimitiveDateTime,
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
