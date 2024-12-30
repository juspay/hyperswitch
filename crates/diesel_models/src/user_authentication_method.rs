use common_utils::encryption::Encryption;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::{enums, schema::user_authentication_methods};

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = user_authentication_methods, check_for_backend(diesel::pg::Pg))]
pub struct UserAuthenticationMethod {
    pub id: String,
    pub auth_id: String,
    pub owner_id: String,
    pub owner_type: enums::Owner,
    pub auth_type: enums::UserAuthType,
    pub private_config: Option<Encryption>,
    pub public_config: Option<serde_json::Value>,
    pub allow_signup: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub email_domain: String,
}

#[derive(router_derive::Setter, Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = user_authentication_methods)]
pub struct UserAuthenticationMethodNew {
    pub id: String,
    pub auth_id: String,
    pub owner_id: String,
    pub owner_type: enums::Owner,
    pub auth_type: enums::UserAuthType,
    pub private_config: Option<Encryption>,
    pub public_config: Option<serde_json::Value>,
    pub allow_signup: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub email_domain: String,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = user_authentication_methods)]
pub struct OrgAuthenticationMethodUpdateInternal {
    pub private_config: Option<Encryption>,
    pub public_config: Option<serde_json::Value>,
    pub last_modified_at: PrimitiveDateTime,
    pub email_domain: Option<String>,
}

pub enum UserAuthenticationMethodUpdate {
    UpdateConfig {
        private_config: Option<Encryption>,
        public_config: Option<serde_json::Value>,
    },
    EmailDomain {
        email_domain: String,
    },
}

impl From<UserAuthenticationMethodUpdate> for OrgAuthenticationMethodUpdateInternal {
    fn from(value: UserAuthenticationMethodUpdate) -> Self {
        let last_modified_at = common_utils::date_time::now();
        match value {
            UserAuthenticationMethodUpdate::UpdateConfig {
                private_config,
                public_config,
            } => Self {
                private_config,
                public_config,
                last_modified_at,
                email_domain: None,
            },
            UserAuthenticationMethodUpdate::EmailDomain { email_domain } => Self {
                private_config: None,
                public_config: None,
                last_modified_at,
                email_domain: Some(email_domain),
            },
        }
    }
}
