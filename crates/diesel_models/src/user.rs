use common_utils::pii;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use masking::Secret;
use time::PrimitiveDateTime;

use crate::schema::users;

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
}

impl From<UserUpdate> for UserUpdateInternal {
        /// Converts a UserUpdate enum into a User struct, setting the appropriate fields based on the variant of the enum.
    fn from(user_update: UserUpdate) -> Self {
        let last_modified_at = common_utils::date_time::now();
        match user_update {
            UserUpdate::VerifyUser => Self {
                name: None,
                password: None,
                is_verified: Some(true),
                last_modified_at,
                preferred_merchant_id: None,
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
            },
        }
    }
}
