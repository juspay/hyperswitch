use diesel::{AsChangeset, Identifiable, Insertable, Queryable};

use crate::schema::organization;

#[derive(Clone, Debug, Identifiable, Queryable)]
#[diesel(table_name = organization, primary_key(org_id))]
pub struct Organization {
    pub org_id: String,
    pub org_name: Option<String>,
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = organization, primary_key(org_id))]
pub struct OrganizationNew {
    pub org_id: String,
    pub org_name: Option<String>,
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = organization)]
pub struct OrganizationUpdateInternal {
    org_name: Option<String>,
}

pub enum OrganizationUpdate {
    Update { org_name: Option<String> },
}

impl From<OrganizationUpdate> for OrganizationUpdateInternal {
    fn from(value: OrganizationUpdate) -> Self {
        match value {
            OrganizationUpdate::Update { org_name } => Self { org_name },
        }
    }
}
