use diesel::{AsChangeset, Identifiable, Insertable, Queryable};
use time::PrimitiveDateTime;

use crate::{enums, schema::roles};

#[derive(Clone, Debug, Identifiable, Queryable)]
#[diesel(table_name = roles)]
pub struct Role {
    pub id: i32,
    pub role_name: String,
    pub role_id: String,
    pub merchant_id: String,
    pub org_id: String,
    #[diesel(deserialize_as = super::DieselArray<enums::PermissionGroup>)]
    pub groups: Vec<enums::PermissionGroup>,
    pub scope: enums::RoleScope,
    pub created_at: PrimitiveDateTime,
    pub created_by: String,
    pub last_modified_at: PrimitiveDateTime,
    pub last_modified_by: String,
}

#[derive(router_derive::Setter, Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = roles)]
pub struct RoleNew {
    pub role_name: String,
    pub role_id: String,
    pub merchant_id: String,
    pub org_id: String,
    #[diesel(deserialize_as = super::DieselArray<enums::PermissionGroup>)]
    pub groups: Vec<enums::PermissionGroup>,
    pub scope: enums::RoleScope,
    pub created_at: PrimitiveDateTime,
    pub created_by: String,
    pub last_modified_at: PrimitiveDateTime,
    pub last_modified_by: String,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = roles)]
pub struct RoleUpdateInternal {
    groups: Option<Vec<enums::PermissionGroup>>,
    role_name: Option<String>,
    last_modified_by: String,
    last_modified_at: PrimitiveDateTime,
}

pub enum RoleUpdate {
    UpdateDetails {
        groups: Option<Vec<enums::PermissionGroup>>,
        role_name: Option<String>,
        last_modified_at: PrimitiveDateTime,
        last_modified_by: String,
    },
}

impl From<RoleUpdate> for RoleUpdateInternal {
    fn from(value: RoleUpdate) -> Self {
        match value {
            RoleUpdate::UpdateDetails {
                groups,
                role_name,
                last_modified_by,
                last_modified_at,
            } => Self {
                groups,
                role_name,
                last_modified_at,
                last_modified_by,
            },
        }
    }
}
