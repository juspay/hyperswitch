use diesel::{deserialize::FromSqlRow, expression::AsExpression};

use crate::id_type;

/// Struct for lineageContext
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize, AsExpression, FromSqlRow)]
#[diesel(sql_type = diesel::sql_types::Jsonb)]
pub struct LineageContext {
    /// user_id: String
    pub user_id: String,

    /// merchant_id: MerchantId
    pub merchant_id: id_type::MerchantId,

    /// role_id: String
    pub role_id: String,

    /// org_id: OrganizationId
    pub org_id: id_type::OrganizationId,

    /// profile_id: ProfileId
    pub profile_id: id_type::ProfileId,

    /// tenant_id: TenantId
    pub tenant_id: id_type::TenantId,
}

crate::impl_to_sql_from_sql_json!(LineageContext);
