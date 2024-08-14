use common_enums::PermissionGroup;
use common_utils::pii;

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
}

#[derive(Debug, serde::Serialize)]
pub enum PermissionModule {
    Payments,
    Refunds,
    MerchantAccount,
    Connectors,
    Routing,
    Analytics,
    Mandates,
    Customer,
    Disputes,
    ThreeDsDecisionManager,
    SurchargeDecisionManager,
    AccountCreate,
    Payouts,
}

#[derive(Debug, serde::Serialize)]
pub struct AuthorizationInfoResponse(pub Vec<AuthorizationInfo>);

#[derive(Debug, serde::Serialize)]
#[serde(untagged)]
pub enum AuthorizationInfo {
    Module(ModuleInfo),
    Group(GroupInfo),
    GroupWithTag(ParentInfo),
}

#[derive(Debug, serde::Serialize)]
pub struct ModuleInfo {
    pub module: PermissionModule,
    pub description: &'static str,
    pub permissions: Vec<PermissionInfo>,
}

#[derive(Debug, serde::Serialize)]
pub struct GroupInfo {
    pub group: PermissionGroup,
    pub description: &'static str,
    pub permissions: Vec<PermissionInfo>,
}

#[derive(Debug, serde::Serialize, Clone)]
pub struct ParentInfo {
    pub name: ParentGroup,
    pub description: &'static str,
    pub groups: Vec<PermissionGroup>,
}

#[derive(Debug, serde::Serialize)]
pub struct PermissionInfo {
    pub enum_name: Permission,
    pub description: &'static str,
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
pub struct MerchantSelectRequest {
    pub merchant_ids: Vec<common_utils::id_type::MerchantId>,
    // TODO: Remove this once the token only api is being used
    pub need_dashboard_entry_response: Option<bool>,
}
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AcceptInvitationRequest {
    pub merchant_ids: Vec<common_utils::id_type::MerchantId>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DeleteUserRoleRequest {
    pub email: pii::Email,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct TransferOrgOwnershipRequest {
    pub email: pii::Email,
}
