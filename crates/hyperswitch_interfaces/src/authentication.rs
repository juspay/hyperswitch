//! Authentication interface

/// struct ExternalAuthenticationPayload
#[derive(Clone, serde::Deserialize, Debug, serde::Serialize, PartialEq, Eq)]
pub struct ExternalAuthenticationPayload {
    /// trans_status
    pub trans_status: common_enums::TransactionStatus,
    /// authentication_value
    pub authentication_value: Option<String>,
    /// eci
    pub eci: Option<String>,
}
