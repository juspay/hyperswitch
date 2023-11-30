use common_utils::pii;
use masking::Secret;

#[derive(Debug, serde::Serialize)]
pub struct ListRolesResponse(pub Vec<RoleInfoResponse>);

#[derive(Debug, serde::Serialize)]
pub struct RoleInfoResponse {
    pub role_id: &'static str,
    pub permissions: Vec<Permission>,
    pub role_name: &'static str,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct GetRoleRequest {
    pub role_id: String,
}

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
    ForexRead,
    RoutingRead,
    RoutingWrite,
    DisputeRead,
    DisputeWrite,
    MandateRead,
    MandateWrite,
    FileRead,
    FileWrite,
    Analytics,
    ThreeDsDecisionManagerWrite,
    ThreeDsDecisionManagerRead,
    SurchargeDecisionManagerWrite,
    SurchargeDecisionManagerRead,
    UsersRead,
    UsersWrite,
}

#[derive(Debug, serde::Serialize)]
pub enum PermissionModule {
    Payments,
    Refunds,
    MerchantAccount,
    Forex,
    Connectors,
    Routing,
    Analytics,
    Mandates,
    Disputes,
    Files,
    ThreeDsDecisionManager,
    SurchargeDecisionManager,
}

#[derive(Debug, serde::Serialize)]
pub struct AuthorizationInfoResponse(pub Vec<ModuleInfo>);

#[derive(Debug, serde::Serialize)]
pub struct ModuleInfo {
    pub module: PermissionModule,
    pub description: &'static str,
    pub permissions: Vec<PermissionInfo>,
}

#[derive(Debug, serde::Serialize)]
pub struct PermissionInfo {
    pub enum_name: Permission,
    pub description: &'static str,
}

#[derive(Debug, serde::Serialize)]
pub struct GetUsersResponse(pub Vec<UserDetails>);

#[derive(Debug, serde::Serialize)]
pub enum UserStatus {
    Active,
    InvitationSent,
}

#[derive(Debug, serde::Serialize)]
pub struct UserDetails {
    pub user_id: String,
    pub email: pii::Email,
    pub name: Secret<String>,
    pub role_id: String,
    pub role_name: String,
    pub status: UserStatus,
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub last_modified_at: time::PrimitiveDateTime,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct InviteUserRequest {
    pub email: pii::Email,
    pub name: Secret<String>,
    pub role_id: String,
}
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct ReInviteUserRequest {
    pub user_id: String,
}

#[derive(Debug, serde::Serialize)]
pub struct InviteUserResponse {
    pub is_email_sent: bool,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdateUserRoleRequest {
    pub user_id: String,
    pub role_id: String,
}
