use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{enums, schema::org_authentication_methods};

#[derive(Clone, Debug, Identifiable, Queryable)]
#[diesel(table_name = org_authentication_methods)]
pub struct OrgAuthenticationMethod {
    pub id: i32,
    pub owner_id: String,
    pub auth_method: enums::AuthMethod,
    pub config: Option<serde_json::Value>,
    pub allow_signup: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(router_derive::Setter, Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = org_authentication_methods)]
pub struct OrgAuthenticationMethodNew {
    pub owner_id: String,
    pub auth_method: enums::AuthMethod,
    pub config: Option<serde_json::Value>,
    pub allow_signup: bool,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = org_authentication_methods)]
pub struct OrgAuthenticationMethodUpdateInternal {
    pub config: Option<serde_json::Value>,
    pub last_modified_at: PrimitiveDateTime,
}

pub enum OrgAuthenticationMethodUpdate {
    UpdateConfig { config: Option<serde_json::Value> },
}

impl From<OrgAuthenticationMethodUpdate> for OrgAuthenticationMethodUpdateInternal {
    fn from(value: OrgAuthenticationMethodUpdate) -> Self {
        let last_modified_at = common_utils::date_time::now();
        match value {
            OrgAuthenticationMethodUpdate::UpdateConfig { config } => Self {
                config,
                last_modified_at,
            },
        }
    }
}
