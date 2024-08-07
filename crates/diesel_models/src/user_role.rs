use common_enums::EntityType;
use common_utils::id_type;
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::{enums, schema::user_roles};

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = user_roles, check_for_backend(diesel::pg::Pg))]
pub struct UserRole {
    pub id: i32,
    pub user_id: String,
    pub merchant_id: Option<id_type::MerchantId>,
    pub role_id: String,
    pub org_id: Option<id_type::OrganizationId>,
    pub status: enums::UserStatus,
    pub created_by: String,
    pub last_modified_by: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub profile_id: Option<String>,
    pub entity_id: Option<String>,
    pub entity_type: Option<EntityType>,
    pub version: enums::UserRoleVersion,
}

#[derive(router_derive::Setter, Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = user_roles)]
pub struct UserRoleNew {
    pub user_id: String,
    pub merchant_id: Option<id_type::MerchantId>,
    pub role_id: String,
    pub org_id: Option<id_type::OrganizationId>,
    pub status: enums::UserStatus,
    pub created_by: String,
    pub last_modified_by: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub profile_id: Option<String>,
    pub entity_id: Option<String>,
    pub entity_type: Option<EntityType>,
    pub version: enums::UserRoleVersion,
}

#[derive(Clone)]
pub struct NewUserRole {
    pub user_id: String,
    pub merchant_id: id_type::MerchantId,
    pub role_id: String,
    pub org_id: id_type::OrganizationId,
    pub status: enums::UserStatus,
    pub created_by: String,
    pub last_modified_by: String,
    pub created_at: PrimitiveDateTime,
    pub last_modified: PrimitiveDateTime,
    pub entity_type: EntityType,
}

impl NewUserRole {
    pub fn to_v1_role(self) -> Option<UserRoleNew> {
        if matches!(self.entity_type, EntityType::Profile) {
            return None;
        }
        Some(UserRoleNew {
            user_id: self.user_id,
            merchant_id: Some(self.merchant_id),
            role_id: self.role_id,
            org_id: Some(self.org_id),
            status: self.status,
            created_by: self.created_by,
            last_modified_by: self.last_modified_by,
            created_at: self.created_at,
            last_modified: self.last_modified,
            profile_id: None,
            entity_id: None,
            entity_type: None,
            version: enums::UserRoleVersion::V1,
        })
    }

    pub fn to_v2_role(self) -> UserRoleNew {
        let merchant_id = match self.entity_type {
            EntityType::Internal => Some(self.merchant_id.to_owned()),
            EntityType::Organization => None,
            EntityType::Merchant => Some(self.merchant_id.to_owned()),
            EntityType::Profile => Some(self.merchant_id.to_owned()),
        };
        let org_id = match self.entity_type {
            EntityType::Internal => Some(self.org_id.to_owned()),
            EntityType::Organization => None,
            EntityType::Merchant => Some(self.org_id.to_owned()),
            EntityType::Profile => Some(self.org_id.to_owned()),
        };
        let profile_id = match self.entity_type {
            EntityType::Internal => None,
            EntityType::Organization => None,
            EntityType::Merchant => None,
            EntityType::Profile => None,
        };
        let entity_id = match self.entity_type {
            EntityType::Internal => Some(self.merchant_id.get_string_repr().to_owned()),
            EntityType::Organization => Some(self.org_id.get_string_repr().to_owned()),
            EntityType::Merchant => Some(self.merchant_id.get_string_repr().to_owned()),
            EntityType::Profile => None,
        };

        UserRoleNew {
            user_id: self.user_id,
            merchant_id,
            role_id: self.role_id,
            org_id,
            status: self.status,
            created_by: self.created_by,
            last_modified_by: self.last_modified_by,
            created_at: self.created_at,
            last_modified: self.last_modified,
            profile_id,
            entity_id,
            entity_type: Some(self.entity_type),
            version: enums::UserRoleVersion::V2,
        }
    }
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = user_roles)]
pub struct UserRoleUpdateInternal {
    role_id: Option<String>,
    status: Option<enums::UserStatus>,
    last_modified_by: Option<String>,
    last_modified: PrimitiveDateTime,
}

pub enum UserRoleUpdate {
    UpdateStatus {
        status: enums::UserStatus,
        modified_by: String,
    },
    UpdateRole {
        role_id: String,
        modified_by: String,
    },
}

impl From<UserRoleUpdate> for UserRoleUpdateInternal {
    fn from(value: UserRoleUpdate) -> Self {
        let last_modified = common_utils::date_time::now();
        match value {
            UserRoleUpdate::UpdateRole {
                role_id,
                modified_by,
            } => Self {
                role_id: Some(role_id),
                last_modified_by: Some(modified_by),
                status: None,
                last_modified,
            },
            UserRoleUpdate::UpdateStatus {
                status,
                modified_by,
            } => Self {
                status: Some(status),
                last_modified,
                last_modified_by: Some(modified_by),
                role_id: None,
            },
        }
    }
}
