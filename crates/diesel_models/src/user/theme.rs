use common_enums::EntityType;
use common_utils::{date_time, id_type, types::theme::ThemeLineage};
use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::schema::themes;

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = themes, primary_key(theme_id), check_for_backend(diesel::pg::Pg))]
pub struct Theme {
    pub theme_id: String,
    pub tenant_id: id_type::TenantId,
    pub org_id: Option<id_type::OrganizationId>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub profile_id: Option<id_type::ProfileId>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub entity_type: EntityType,
    pub theme_name: String,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = themes)]
pub struct ThemeNew {
    pub theme_id: String,
    pub tenant_id: id_type::TenantId,
    pub org_id: Option<id_type::OrganizationId>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub profile_id: Option<id_type::ProfileId>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
    pub entity_type: EntityType,
    pub theme_name: String,
}

impl ThemeNew {
    pub fn new(theme_id: String, theme_name: String, lineage: ThemeLineage) -> Self {
        let now = date_time::now();

        Self {
            theme_id,
            theme_name,
            tenant_id: lineage.tenant_id().to_owned(),
            org_id: lineage.org_id().cloned(),
            merchant_id: lineage.merchant_id().cloned(),
            profile_id: lineage.profile_id().cloned(),
            entity_type: lineage.entity_type(),
            created_at: now,
            last_modified_at: now,
        }
    }
}
