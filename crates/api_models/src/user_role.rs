use common_enums::PermissionGroup;
use common_utils::pii;
use masking::Secret;

pub mod role;

#[derive(Debug, serde::Serialize)]
pub enum Permission {
    PaymentRead,
    PaymentWrite,
    RefundRead,
    RefundWrite,
    ApiKeyRead,
    ApiKeyWrite,
    MerchantAccountRead,
    MerchantAccountWrite,
    MerchantConnectorAccountRead,
    MerchantConnectorAccountWrite,
    RoutingRead,
    RoutingWrite,
    DisputeRead,
    DisputeWrite,
    MandateRead,
    MandateWrite,
    CustomerRead,
    CustomerWrite,
    Analytics,
    ThreeDsDecisionManagerWrite,
    ThreeDsDecisionManagerRead,
    SurchargeDecisionManagerWrite,
    SurchargeDecisionManagerRead,
    UsersRead,
    UsersWrite,
    MerchantAccountCreate,
    WebhookEventRead,
    PayoutWrite,
    PayoutRead,
    WebhookEventWrite,
    GenerateReport,
    ReconAdmin,
}

#[derive(Clone, Debug, serde::Serialize, PartialEq, Eq, Hash)]
pub enum ParentGroup {
    Operations,
    Connectors,
    Workflows,
    Analytics,
    Users,
    #[serde(rename = "MerchantAccess")]
    Merchant,
    #[serde(rename = "OrganizationAccess")]
    Organization,
    Recon,
}

#[derive(Debug, serde::Serialize)]
pub struct AuthorizationInfoResponse(pub Vec<ParentInfo>);

#[derive(Debug, serde::Serialize, Clone)]
pub struct ParentInfo {
    pub name: ParentGroup,
    pub description: &'static str,
    pub groups: Vec<PermissionGroup>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdateUserRoleRequest {
    pub email: pii::Email,
    pub role_id: String,
}

#[derive(Debug, serde::Serialize)]
pub enum UserStatus {
    Active,
    InvitationSent,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DeleteUserRoleRequest {
    pub email: pii::Email,
}

#[derive(Debug, serde::Serialize)]
pub struct ListUsersInEntityResponse {
    pub email: pii::Email,
    pub roles: Vec<role::MinimalRoleInfo>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ListInvitationForUserResponse {
    pub entity_id: String,
    pub entity_type: common_enums::EntityType,
    pub entity_name: Option<Secret<String>>,
    pub role_id: String,
}

pub type AcceptInvitationsV2Request = Vec<Entity>;
pub type AcceptInvitationsPreAuthRequest = Vec<Entity>;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct Entity {
    pub entity_id: String,
    pub entity_type: common_enums::EntityType,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ListUsersInEntityRequest {
    pub entity_type: Option<common_enums::EntityType>,
}
