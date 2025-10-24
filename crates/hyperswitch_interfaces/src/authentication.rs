//! Authentication interface

/// struct ExternalAuthenticationPayload
#[derive(Clone, serde::Deserialize, Debug, serde::Serialize, PartialEq, Eq)]
pub struct ExternalAuthenticationPayload {
    /// trans_status
    pub trans_status: common_enums::TransactionStatus,
    /// authentication_value
    pub authentication_value: Option<masking::Secret<String>>,
    /// eci
    pub eci: Option<String>,
    /// Indicates whether the challenge was canceled by the user or system.
    pub challenge_cancel: Option<String>,
    /// Reason for the challenge code, if applicable.
    pub challenge_code_reason: Option<String>,
}
