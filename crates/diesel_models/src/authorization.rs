use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::incremental_authorization};

#[derive(Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Serialize, Deserialize, Hash)]
#[diesel(table_name = incremental_authorization)]
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
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub connector_authorization_id: Option<String>,
    pub previously_authorized_amount: i64,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize)]
#[diesel(table_name = incremental_authorization)]
pub struct AuthorizationNew {
    pub authorization_id: String,
    pub merchant_id: String,
    pub payment_id: String,
    pub amount: i64,
    pub status: storage_enums::AuthorizationStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub connector_authorization_id: Option<String>,
    pub previously_authorized_amount: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthorizationUpdate {
    StatusUpdate {
        status: storage_enums::AuthorizationStatus,
        error_code: Option<String>,
        error_message: Option<String>,
        connector_authorization_id: Option<String>,
    },
}

#[derive(Clone, Debug, Default, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = incremental_authorization)]
pub struct AuthorizationUpdateInternal {
    pub status: Option<storage_enums::AuthorizationStatus>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub modified_at: Option<PrimitiveDateTime>,
    pub connector_authorization_id: Option<String>,
}

impl AuthorizationUpdateInternal {
        /// Creates a new authorization by combining the properties of the current authorization and the provided source authorization.
    pub fn create_authorization(self, source: Authorization) -> Authorization {
        Authorization {
            status: self.status.unwrap_or(source.status),
            error_code: self.error_code.or(source.error_code),
            error_message: self.error_message.or(source.error_message),
            modified_at: self.modified_at.unwrap_or(common_utils::date_time::now()),
            connector_authorization_id: self
                .connector_authorization_id
                .or(source.connector_authorization_id),
            ..source
        }
    }
}

impl From<AuthorizationUpdate> for AuthorizationUpdateInternal {
        /// Constructs a new instance of Self (Authorization) based on the provided AuthorizationUpdate.
    fn from(authorization_child_update: AuthorizationUpdate) -> Self {
        let now = Some(common_utils::date_time::now());
        match authorization_child_update {
            AuthorizationUpdate::StatusUpdate {
                status,
                error_code,
                error_message,
                connector_authorization_id,
            } => Self {
                status: Some(status),
                error_code,
                error_message,
                connector_authorization_id,
                modified_at: now,
            },
        }
    }
}
