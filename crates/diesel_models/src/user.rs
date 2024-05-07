use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, enums::TotpStatus, schema::users};

pub mod dashboard_metadata;

pub mod sample_data;
#[derive(Clone, Debug, Identifiable, Queryable)]
#[diesel(table_name = users)]
pub struct User {
    pub id: i32,
    pub user_id: String,
    pub email: pii::Email,
    pub name: Secret<String>,
    pub password: Secret<String>,
    pub is_verified: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub preferred_merchant_id: Option<String>,
    pub totp_status: TotpStatus,
    pub totp_secret: Option<Encryption>,
    pub totp_recovery_codes: Option<serde_json::Value>,
}

#[derive(
    router_derive::Setter, Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay,
)]
#[diesel(table_name = users)]
pub struct UserNew {
    pub user_id: String,
    pub email: pii::Email,
    pub name: Secret<String>,
    pub password: Secret<String>,
    pub is_verified: bool,
    pub created_at: Option<PrimitiveDateTime>,
    pub last_modified_at: Option<PrimitiveDateTime>,
    pub preferred_merchant_id: Option<String>,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = users)]
pub struct UserUpdateInternal {
    name: Option<String>,
    password: Option<Secret<String>>,
    is_verified: Option<bool>,
    last_modified_at: PrimitiveDateTime,
    preferred_merchant_id: Option<String>,
    totp_status: Option<TotpStatus>,
    totp_secret: Option<Encryption>,
    totp_recovery_codes: Option<serde_json::Value>,
}

#[derive(Debug)]
pub enum UserUpdate {
    VerifyUser,
    AccountUpdate {
        name: Option<String>,
        password: Option<Secret<String>>,
        is_verified: Option<bool>,
        preferred_merchant_id: Option<String>,
    },
    TotpUpdate {
        totp_status: Option<TotpStatus>,
        totp_secret: Option<Encryption>,
        totp_recovery_codes: Option<serde_json::Value>,
    },
}

impl From<UserUpdate> for UserUpdateInternal {
    fn from(user_update: UserUpdate) -> Self {
        let last_modified_at = common_utils::date_time::now();
        match user_update {
            UserUpdate::VerifyUser => Self {
                name: None,
                password: None,
                is_verified: Some(true),
                last_modified_at,
                preferred_merchant_id: None,
                totp_status: None,
                totp_secret: None,
                totp_recovery_codes: None,
            },
            UserUpdate::AccountUpdate {
                name,
                password,
                is_verified,
                preferred_merchant_id,
            } => Self {
                name,
                password,
                is_verified,
                last_modified_at,
                preferred_merchant_id,
                totp_status: None,
                totp_secret: None,
                totp_recovery_codes: None,
            },
            UserUpdate::TotpUpdate {
                totp_status,
                totp_secret,
                totp_recovery_codes,
            } => Self {
                name: None,
                password: None,
                is_verified: None,
                last_modified_at,
                preferred_merchant_id: None,
                totp_status,
                totp_secret,
                totp_recovery_codes,
            },
        }
    }
}
