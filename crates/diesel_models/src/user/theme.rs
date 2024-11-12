use common_utils::id_type;
use diesel::{Identifiable, Insertable, Queryable, Selectable};
use time::PrimitiveDateTime;

use crate::schema::themes;

#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = themes, primary_key(theme_id), check_for_backend(diesel::pg::Pg))]
pub struct Theme {
    pub theme_id: String,
    pub tenant_id: String,
    pub org_id: Option<id_type::OrganizationId>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub profile_id: Option<id_type::ProfileId>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable, router_derive::DebugAsDisplay)]
#[diesel(table_name = themes)]
pub struct ThemeNew {
    pub theme_id: String,
    pub tenant_id: String,
    pub org_id: Option<id_type::OrganizationId>,
    pub merchant_id: Option<id_type::MerchantId>,
    pub profile_id: Option<id_type::ProfileId>,
    pub created_at: PrimitiveDateTime,
    pub last_modified_at: PrimitiveDateTime,
}
