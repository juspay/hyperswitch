use common_utils::types::MinorUnit;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::{enums as storage_enums, schema::incremental_authorization};

#[derive(
    Clone, Debug, Eq, PartialEq, Identifiable, Queryable, Selectable, Serialize, Deserialize, Hash,
)]
#[diesel(table_name = incremental_authorization, primary_key(authorization_id, merchant_id), check_for_backend(diesel::pg::Pg))]
pub struct Authorization {
    pub authorization_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_id: common_utils::id_type::PaymentId,
    pub amount: MinorUnit,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub modified_at: PrimitiveDateTime,
    pub status: storage_enums::AuthorizationStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub connector_authorization_id: Option<String>,
    pub previously_authorized_amount: MinorUnit,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay, Serialize, Deserialize)]
#[diesel(table_name = incremental_authorization)]
pub struct AuthorizationNew {
    pub authorization_id: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    pub payment_id: common_utils::id_type::PaymentId,
    pub amount: MinorUnit,
    pub status: storage_enums::AuthorizationStatus,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub connector_authorization_id: Option<String>,
    pub previously_authorized_amount: MinorUnit,
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
