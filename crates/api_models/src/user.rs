use common_utils::pii;
use masking::Secret;
pub mod dashboard_metadata;
#[cfg(feature = "dummy_connector")]
pub mod sample_data;

#[derive(serde::Deserialize, Debug, Clone, serde::Serialize)]
pub struct SignUpWithMerchantIdRequest {
    pub name: Secret<String>,
    pub email: pii::Email,
    pub password: Secret<String>,
    pub company_name: String,
}

#[derive(serde::Deserialize, Debug, Clone, serde::Serialize)]
pub struct SignUpRequest {
    pub email: pii::Email,
    pub password: Secret<String>,
}

pub type SignUpResponse = DashboardEntryResponse;

#[derive(serde::Serialize, Debug, Clone)]
pub struct DashboardEntryResponse {
    pub token: Secret<String>,
    pub merchant_id: String,
    pub name: Secret<String>,
    pub email: pii::Email,
    pub verification_days_left: Option<i64>,
    pub user_role: String,
    //this field is added for audit/debug reasons
    #[serde(skip_serializing)]
    pub user_id: String,
}

pub type SignInRequest = SignUpRequest;

pub type SignInResponse = DashboardEntryResponse;

#[derive(serde::Deserialize, Debug, Clone, serde::Serialize)]
pub struct ConnectAccountRequest {
    pub email: pii::Email,
}

pub type ConnectAccountResponse = AuthorizeResponse;

#[derive(serde::Serialize, Debug, Clone)]
pub struct AuthorizeResponse {
    pub is_email_sent: bool,
    //this field is added for audit/debug reasons
    #[serde(skip_serializing)]
    pub user_id: String,
    //this field is added for audit/debug reasons
    #[serde(skip_serializing)]
    pub merchant_id: String,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct ChangePasswordRequest {
    pub new_password: Secret<String>,
    pub old_password: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct SwitchMerchantIdRequest {
    pub merchant_id: String,
}

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct CreateInternalUserRequest {
    pub name: Secret<String>,
    pub email: pii::Email,
    pub password: Secret<String>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct UserMerchantCreate {
    pub company_name: String,
}
