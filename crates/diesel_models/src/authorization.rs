use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::authorization};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize, Hash)]
#[diesel(table_name = authorization)]
#[diesel(primary_key(authorization_id, merchant_id))]
pub struct Authorization {
    pub authorization_id: String,
    pub merchant_id: String,
    pub payment_id: String,
    pub amount: i64,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub status: storage_enums::AuthorizationStatus,
    pub code: Option<String>,
    pub message: Option<String>,
    pub connector_authorization_id: Option<String>,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize)]
#[diesel(table_name = authorization)]
pub struct AuthorizationNew {
    pub authorization_id: String,
    pub merchant_id: String,
    pub payment_id: String,
    pub amount: i64,
    pub status: storage_enums::AuthorizationStatus,
    pub code: Option<String>,
    pub message: Option<String>,
    pub connector_authorization_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorizationUpdate {
    StatusUpdate {
        status: storage_enums::AuthorizationStatus,
        code: Option<String>,
        message: Option<String>,
        connector_authorization_id: Option<String>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = authorization)]
pub struct AuthorizationUpdateInternal {
    pub status: Option<storage_enums::AuthorizationStatus>,
    pub code: Option<String>,
    pub message: Option<String>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub connector_authorization_id: Option<String>,
}

impl From<AuthorizationUpdate> for AuthorizationUpdateInternal {
    fn from(authorization_child_update: AuthorizationUpdate) -> Self {
        let now = Some(common_utils::date_time::now());
        match authorization_child_update {
            AuthorizationUpdate::StatusUpdate {
                status,
                code,
                message,
                connector_authorization_id,
            } => Self {
                status: Some(status),
                code,
                message,
                connector_authorization_id,
                modified_at: now,
            },
        }
    }
}
