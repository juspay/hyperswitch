use common_utils::{encryption::Encryption, pii, types::user::LineageContext};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use masking::Secret;
use time::PrimitiveDateTime;

use crate::{diesel_impl::OptionalDieselArray, enums::TotpStatus, schema::users};

pub mod dashboard_metadata;
pub mod sample_data;
pub mod theme;

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = users, primary_key(user_id), check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub user_id: String,
    pub email: pii::Email,
    pub name: Secret<String>,
    pub password: Option<Secret<String>>,
    pub is_verified: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub totp_status: TotpStatus,
    pub totp_secret: Option<Encryption>,
    #[diesel(deserialize_as = OptionalDieselArray<Secret<String>>)]
    pub totp_recovery_codes: Option<Vec<Secret<String>>>,
    pub last_password_modified_at: Option<PrimitiveDateTime>,
    pub lineage_context: Option<LineageContext>,
}

#[derive(
    router_derive::Setter, Clone, Debug, Default, Insertable, router_derive::DebugAsDisplay,
)]
#[diesel(table_name = users)]
pub struct UserNew {
    pub user_id: String,
    pub email: pii::Email,
    pub name: Secret<String>,
    pub password: Option<Secret<String>>,
    pub is_verified: bool,
    pub created_at: Option<PrimitiveDateTime>,
    pub last_modified_at: Option<PrimitiveDateTime>,
    pub totp_status: TotpStatus,
    pub totp_secret: Option<Encryption>,
    pub totp_recovery_codes: Option<Vec<Secret<String>>>,
    pub last_password_modified_at: Option<PrimitiveDateTime>,
    pub lineage_context: Option<LineageContext>,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = users)]
pub struct UserUpdateInternal {
    name: Option<String>,
    password: Option<Secret<String>>,
    is_verified: Option<bool>,
    last_modified_at: PrimitiveDateTime,
    totp_status: Option<TotpStatus>,
    totp_secret: Option<Encryption>,
    totp_recovery_codes: Option<Vec<Secret<String>>>,
    last_password_modified_at: Option<PrimitiveDateTime>,
    lineage_context: Option<LineageContext>,
}

#[derive(Debug)]
pub enum UserUpdate {
    VerifyUser,
    AccountUpdate {
        name: Option<String>,
        is_verified: Option<bool>,
    },
    TotpUpdate {
        totp_status: Option<TotpStatus>,
        totp_secret: Option<Encryption>,
        totp_recovery_codes: Option<Vec<Secret<String>>>,
    },
    PasswordUpdate {
        password: Secret<String>,
    },
    LineageContextUpdate {
        lineage_context: LineageContext,
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
                totp_status: None,
                totp_secret: None,
                totp_recovery_codes: None,
                last_password_modified_at: None,
                lineage_context: None,
            },
            UserUpdate::AccountUpdate { name, is_verified } => Self {
                name,
                password: None,
                is_verified,
                last_modified_at,
                totp_status: None,
                totp_secret: None,
                totp_recovery_codes: None,
                last_password_modified_at: None,
                lineage_context: None,
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
                totp_status,
                totp_secret,
                totp_recovery_codes,
                last_password_modified_at: None,
                lineage_context: None,
            },
            UserUpdate::PasswordUpdate { password } => Self {
                name: None,
                password: Some(password),
                is_verified: None,
                last_modified_at,
                last_password_modified_at: Some(last_modified_at),
                totp_status: None,
                totp_secret: None,
                totp_recovery_codes: None,
                lineage_context: None,
            },
            UserUpdate::LineageContextUpdate { lineage_context } => Self {
                name: None,
                password: None,
                is_verified: None,
                last_modified_at,
                last_password_modified_at: None,
                totp_status: None,
                totp_secret: None,
                totp_recovery_codes: None,
                lineage_context: Some(lineage_context),
            },
        }
    }
}
