#[derive(Clone, serde::Deserialize, Debug, serde::Serialize, PartialEq, Eq)]
pub struct ExternalAuthenticationPayload {
    pub trans_status: common_enums::TransactionStatus,
    pub authentication_value: Option<String>,
    pub eci: Option<String>,
}
