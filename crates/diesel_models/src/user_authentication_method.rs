use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{encryption::Encryption, enums, schema::user_authentication_methods};

#[derive(Clone, Debug, Identifiable, Queryable)]
#[diesel(table_name = user_authentication_methods)]
pub struct UserAuthenticationMethod {
    pub id: String,
    pub auth_id: String,
    pub owner_id: String,
    pub owner_type: enums::Owner,
    pub auth_method: enums::AuthMethod,
    pub config: Option<Encryption>,
    pub allow_signup: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(router_derive::Setter, Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = user_authentication_methods)]
pub struct UserAuthenticationMethodNew {
    pub id: String,
    pub auth_id: String,
    pub owner_id: String,
    pub owner_type: enums::Owner,
    pub auth_method: enums::AuthMethod,
    pub config: Option<Encryption>,
    pub allow_signup: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = user_authentication_methods)]
pub struct OrgAuthenticationMethodUpdateInternal {
    pub config: Option<Encryption>,
    pub last_modified_at: PrimitiveDateTime,
}

pub enum UserAuthenticationMethodUpdate {
    UpdateConfig { config: Option<Encryption> },
}

impl From<UserAuthenticationMethodUpdate> for OrgAuthenticationMethodUpdateInternal {
    fn from(value: UserAuthenticationMethodUpdate) -> Self {
        let last_modified_at = common_utils::date_time::now();
        match value {
            UserAuthenticationMethodUpdate::UpdateConfig { config } => Self {
                config,
                last_modified_at,
            },
        }
    }
}
