use common_utils::pii;
use masking::Secret;

#[derive(serde::Deserialize, Debug, Clone, serde::Serialize)]
pub struct ConnectAccountRequest {
    pub email: pii::Email,
    pub password: Secret<String>,
}

#[derive(serde::Serialize, Debug, Clone)]
pub struct ConnectAccountResponse {
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

#[derive(serde::Deserialize, Debug, serde::Serialize)]
pub struct ChangePasswordRequest {
    pub new_password: Secret<String>,
    pub old_password: Secret<String>,
}
