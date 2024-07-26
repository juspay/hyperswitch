use common_utils::{id_type, pii};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

use crate::schema::organization;
#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(table_name = organization, primary_key(org_id), check_for_backend(diesel::pg::Pg))]
pub struct Organization {
    pub org_id: id_type::OrganizationId,
    pub org_name: Option<String>,
    pub organization_details: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = organization, primary_key(org_id))]
pub struct OrganizationNew {
    pub org_id: id_type::OrganizationId,
    pub org_name: Option<String>,
    pub organization_details: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = organization)]
pub struct OrganizationUpdateInternal {
    org_name: Option<String>,
    organization_details: Option<pii::SecretSerdeValue>,
    metadata: Option<pii::SecretSerdeValue>,
    modified_at: time::PrimitiveDateTime,
}

pub enum OrganizationUpdate {
    Update {
        org_name: Option<String>,
        organization_details: Option<pii::SecretSerdeValue>,
        metadata: Option<pii::SecretSerdeValue>,
    },
}

impl From<OrganizationUpdate> for OrganizationUpdateInternal {
    fn from(value: OrganizationUpdate) -> Self {
        match value {
            OrganizationUpdate::Update {
                org_name,
                organization_details,
                metadata,
            } => Self {
                org_name,
                organization_details,
                metadata,
                modified_at: common_utils::date_time::now(),
            },
        }
    }
}
