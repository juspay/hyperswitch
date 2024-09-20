use common_utils::{id_type, pii};
use utoipa::ToSchema;
pub struct OrganizationNew {
    pub org_id: id_type::OrganizationId,
    pub org_name: Option<String>,
}

impl OrganizationNew {
    pub fn new(org_name: Option<String>) -> Self {
        Self {
            org_id: id_type::OrganizationId::default(),
            org_name,
        }
    }
}

#[derive(Clone, Debug, serde::Serialize)]
pub struct OrganizationId {
    pub organization_id: id_type::OrganizationId,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationCreateRequest {
    pub organization_name: String,
    #[schema(value_type = Option<Object>)]
    pub organization_details: Option<pii::SecretSerdeValue>,
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct OrganizationUpdateRequest {
    pub organization_name: Option<String>,
    #[schema(value_type = Option<Object>)]
    pub organization_details: Option<pii::SecretSerdeValue>,
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<pii::SecretSerdeValue>,
}

#[derive(Debug, serde::Serialize, Clone, ToSchema)]
pub struct OrganizationResponse {
    #[schema(value_type = String, max_length = 64, min_length = 1, example = "org_q98uSGAYbjEwqs0mJwnz")]
    pub organization_id: id_type::OrganizationId,
    pub organization_name: Option<String>,
    #[schema(value_type = Option<Object>)]
    pub organization_details: Option<pii::SecretSerdeValue>,
    #[schema(value_type = Option<Object>)]
    pub metadata: Option<pii::SecretSerdeValue>,
    pub modified_at: time::PrimitiveDateTime,
    pub created_at: time::PrimitiveDateTime,
}
