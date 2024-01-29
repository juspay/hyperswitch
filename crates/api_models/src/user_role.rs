use common_utils::pii;

use crate::user::DashboardEntryResponse;

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
    CustomerRead,
    CustomerWrite,
    FileRead,
    FileWrite,
    Analytics,
    ThreeDsDecisionManagerWrite,
    ThreeDsDecisionManagerRead,
    SurchargeDecisionManagerWrite,
    SurchargeDecisionManagerRead,
    UsersRead,
    UsersWrite,
    MerchantAccountCreate,
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
    Customer,
    Disputes,
    Files,
    ThreeDsDecisionManager,
    SurchargeDecisionManager,
    AccountCreate,
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

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UpdateUserRoleRequest {
    pub user_id: String,
    pub role_id: String,
}

#[derive(Debug, serde::Serialize)]
pub enum UserStatus {
    Active,
    InvitationSent,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct AcceptInvitationRequest {
    pub merchant_ids: Vec<String>,
    pub need_dashboard_entry_response: Option<bool>,
}

pub type AcceptInvitationResponse = DashboardEntryResponse;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct DeleteUserRoleRequest {
    pub email: pii::Email,
}
