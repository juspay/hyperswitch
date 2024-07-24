use common_utils::{id_type, pii};
use diesel::{AsChangeset, Identifiable, Insertable, Queryable, Selectable};

#[cfg(feature = "v1")]
use crate::schema::organization;
#[cfg(feature = "v2")]
use crate::schema_v2::organization;
pub trait OrganizationBridge {
    fn get_organization_id(&self) -> id_type::OrganizationId;
    fn get_organization_name(&self) -> Option<String>;
    fn set_organization_name(&mut self, organization_name: String);
}
#[cfg(feature = "v1")]
#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(
    table_name = organization,
    primary_key(org_id),
    check_for_backend(diesel::pg::Pg)
)]
pub struct Organization {
    org_id: id_type::OrganizationId,
    org_name: Option<String>,
    pub organization_details: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

#[cfg(feature = "v2")]
#[derive(Clone, Debug, Identifiable, Queryable, Selectable)]
#[diesel(
    table_name = organization,
    primary_key(id),
    check_for_backend(diesel::pg::Pg)
)]
pub struct Organization {
    id: id_type::OrganizationId,
    organization_name: Option<String>,
    pub organization_details: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

impl Organization {
    pub fn new(org_new: OrganizationNew) -> Self {
        let OrganizationNew {
            #[cfg(feature = "v1")]
            org_id,
            #[cfg(feature = "v1")]
            org_name,
            #[cfg(feature = "v2")]
            id,
            #[cfg(feature = "v2")]
            organization_name,
            organization_details,
            metadata,
            created_at,
            modified_at,
        } = org_new;
        Self {
            #[cfg(feature = "v1")]
            org_id,
            #[cfg(feature = "v1")]
            org_name,
            #[cfg(feature = "v2")]
            id,
            #[cfg(feature = "v2")]
            organization_name,
            organization_details,
            metadata,
            created_at,
            modified_at,
        }
    }
}

#[derive(Clone, Debug, Insertable)]
#[diesel(table_name = organization, primary_key(org_id))]
pub struct OrganizationNew {
    #[cfg(feature = "v1")]
    org_id: id_type::OrganizationId,
    #[cfg(feature = "v1")]
    org_name: Option<String>,
    #[cfg(feature = "v2")]
    id: id_type::OrganizationId,
    #[cfg(feature = "v2")]
    organization_name: Option<String>,
    pub organization_details: Option<pii::SecretSerdeValue>,
    pub metadata: Option<pii::SecretSerdeValue>,
    pub created_at: time::PrimitiveDateTime,
    pub modified_at: time::PrimitiveDateTime,
}

impl OrganizationNew {
    pub fn new(id: id_type::OrganizationId, organization_name: Option<String>) -> Self {
        Self {
            #[cfg(feature = "v1")]
            org_id: id,
            #[cfg(feature = "v2")]
            id,
            #[cfg(feature = "v1")]
            org_name: organization_name,
            #[cfg(feature = "v2")]
            organization_name,
            organization_details: None,
            metadata: None,
            created_at: common_utils::date_time::now(),
            modified_at: common_utils::date_time::now(),
        }
    }
}

#[derive(Clone, Debug, AsChangeset)]
#[diesel(table_name = organization)]
pub struct OrganizationUpdateInternal {
    #[cfg(feature = "v1")]
    org_name: Option<String>,
    #[cfg(feature = "v2")]
    organization_name: Option<String>,
    organization_details: Option<pii::SecretSerdeValue>,
    metadata: Option<pii::SecretSerdeValue>,
    modified_at: time::PrimitiveDateTime,
}

pub enum OrganizationUpdate {
    Update {
        organization_name: Option<String>,
        organization_details: Option<pii::SecretSerdeValue>,
        metadata: Option<pii::SecretSerdeValue>,
    },
}

impl From<OrganizationUpdate> for OrganizationUpdateInternal {
    fn from(value: OrganizationUpdate) -> Self {
        match value {
            OrganizationUpdate::Update {
                organization_name,
                organization_details,
                metadata,
            } => Self {
                #[cfg(feature = "v1")]
                org_name: organization_name,
                #[cfg(feature = "v2")]
                organization_name,
                organization_details,
                metadata,
                modified_at: common_utils::date_time::now(),
            },
        }
    }
}

impl OrganizationBridge for Organization {
    fn get_organization_id(&self) -> id_type::OrganizationId {
        #[cfg(feature = "v1")]
        {
            self.org_id.clone()
        }
        #[cfg(feature = "v2")]
        {
            self.id.clone()
        }
    }

    fn get_organization_name(&self) -> Option<String> {
        #[cfg(feature = "v1")]
        {
            self.org_name.clone()
        }
        #[cfg(feature = "v2")]
        {
            self.organization_name.clone()
        }
    }
    fn set_organization_name(&mut self, organization_name: String) {
        #[cfg(feature = "v1")]
        {
            self.org_name = Some(organization_name);
        }
        #[cfg(feature = "v2")]
        {
            self.organization_name = Some(organization_name);
        }
    }
}

impl OrganizationBridge for OrganizationNew {
    fn get_organization_id(&self) -> id_type::OrganizationId {
        #[cfg(feature = "v1")]
        {
            self.org_id.clone()
        }
        #[cfg(feature = "v2")]
        {
            self.id.clone()
        }
    }

    fn get_organization_name(&self) -> Option<String> {
        #[cfg(feature = "v1")]
        {
            self.org_name.clone()
        }
        #[cfg(feature = "v2")]
        {
            self.organization_name.clone()
        }
    }
    fn set_organization_name(&mut self, organization_name: String) {
        #[cfg(feature = "v1")]
        {
            self.org_name = Some(organization_name);
        }
        #[cfg(feature = "v2")]
        {
            self.organization_name = Some(organization_name);
        }
    }
}
