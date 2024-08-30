use std::hash::Hash;

use common_enums::EntityType;
use common_utils::{consts, id_type};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::{enums, schema::user_roles};

#[derive(Clone, Debug, Identifiable, Queryable, Selectable, Eq)]
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
    pub profile_id: Option<id_type::ProfileId>,
    pub entity_id: Option<String>,
    pub entity_type: Option<EntityType>,
    pub version: enums::UserRoleVersion,
}

fn get_entity_id_and_type(user_role: &UserRole) -> (Option<String>, Option<EntityType>) {
    match (user_role.version, user_role.role_id.as_str()) {
        (enums::UserRoleVersion::V1, consts::ROLE_ID_ORGANIZATION_ADMIN) => (
            user_role
                .org_id
                .clone()
                .map(|org_id| org_id.get_string_repr().to_string()),
            Some(EntityType::Organization),
        ),
        (enums::UserRoleVersion::V1, consts::ROLE_ID_INTERNAL_VIEW_ONLY_USER)
        | (enums::UserRoleVersion::V1, consts::ROLE_ID_INTERNAL_ADMIN) => (
            user_role
                .merchant_id
                .clone()
                .map(|merchant_id| merchant_id.get_string_repr().to_string()),
            Some(EntityType::Internal),
        ),
        (enums::UserRoleVersion::V1, _) => (
            user_role
                .merchant_id
                .clone()
                .map(|merchant_id| merchant_id.get_string_repr().to_string()),
            Some(EntityType::Merchant),
        ),
        (enums::UserRoleVersion::V2, _) => (user_role.entity_id.clone(), user_role.entity_type),
    }
}

impl Hash for UserRole {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let (entity_id, entity_type) = get_entity_id_and_type(self);

        self.user_id.hash(state);
        entity_id.hash(state);
        entity_type.hash(state);
    }
}

impl PartialEq for UserRole {
    fn eq(&self, other: &Self) -> bool {
        let (self_entity_id, self_entity_type) = get_entity_id_and_type(self);
        let (other_entity_id, other_entity_type) = get_entity_id_and_type(other);

        self.user_id == other.user_id
            && self_entity_id == other_entity_id
            && self_entity_type == other_entity_type
    }
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
    pub profile_id: Option<id_type::ProfileId>,
    pub entity_id: Option<String>,
    pub entity_type: Option<EntityType>,
    pub version: enums::UserRoleVersion,
}

#[derive(Clone, Debug, AsChangeset, router_derive::DebugAsDisplay)]
#[diesel(table_name = user_roles)]
pub struct UserRoleUpdateInternal {
    role_id: Option<String>,
    status: Option<enums::UserStatus>,
    last_modified_by: Option<String>,
    last_modified: PrimitiveDateTime,
}

#[derive(Clone)]
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
