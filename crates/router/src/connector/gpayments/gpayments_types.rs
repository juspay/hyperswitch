use cards::CardNumber;
use common_utils::types;
use masking::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GpaymentsConnectorMetaData {
    pub authentication_url: String,
    pub three_ds_requestor_trans_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpaymentsPreAuthVersionCallRequest {
    pub acct_number: CardNumber,
    pub merchant_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpaymentsPreAuthVersionCallResponse {
    pub enrolment_status: EnrollmentStatus,
    pub supported_message_versions: Option<Vec<types::SemanticVersion>>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EnrollmentStatus {
    #[serde(rename = "00")]
    NotEnrolled,
    #[serde(rename = "01")]
    Enrolled,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TDS2ApiError {
    pub error_code: String,
    pub error_component: Option<String>,
    pub error_description: String,
    pub error_detail: Option<String>,
    pub error_message_type: Option<String>,
    pub message_type: String,
    pub message_version: Option<String>,
    #[serde(rename = "sdkTransID")]
    pub sdk_trans_id: Option<String>,
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpaymentsPreAuthenticationRequest {
    pub acct_number: CardNumber,
    pub card_scheme: Option<CardScheme>,
    pub challenge_window_size: Option<ChallengeWindowSize>,

    /// URL where the 3DS requestor receives events during authentication.
    /// ActiveServer calls this URL through the iframe to notify events occurring during authentication.
    ///
    /// Example: "https://example.requestor.com/3ds-notify"
    /// Length: Maximum 2000 characters
    pub event_callback_url: String,

    /// Merchant identifier assigned by the acquirer.
    /// This may be the same value used in authorization requests sent on behalf of the 3DS Requestor.
    ///
    /// Example: "1234567890123456789012345678901234"
    /// Length: Maximum 35 characters
    pub merchant_id: String,

    /// Optional boolean. If set to true, ActiveServer will not collect the browser information automatically.
    /// The requestor must have a backend implementation to collect browser information.
    pub skip_auto_browser_info_collect: Option<bool>,

    /// Universal unique transaction identifier assigned by the 3DS Requestor to identify a single transaction.
    ///
    /// Example: "6afa6072-9412-446b-9673-2f98b3ee98a2"
    /// Length: 36 characters
    #[serde(rename = "threeDSRequestorTransID")]
    pub three_ds_requestor_trans_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ChallengeWindowSize {
    #[serde(rename = "01")]
    Size250x400,
    #[serde(rename = "02")]
    Size390x400,
    #[serde(rename = "03")]
    Size500x600,
    #[serde(rename = "04")]
    Size600x400,
    #[serde(rename = "05")]
    FullScreen,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "UPPERCASE")]
pub enum CardScheme {
    Visa,
    MasterCard,
    AmericanExpress,
    JCB,
    Discover,
    UnionPay,
    Mir,
    Eftpos,
    BCard,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpaymentsPreAuthenticationResponse {
    pub auth_url: String,
    pub mon_url: Option<String>,
    pub resolved_card_scheme: Option<CardScheme>,
    #[serde(rename = "threeDSMethodAvailable")]
    pub three_ds_method_available: Option<bool>,
    #[serde(rename = "threeDSMethodUrl")]
    pub three_ds_method_url: Option<String>,
    #[serde(rename = "threeDSServerCallbackUrl")]
    pub three_ds_server_callback_url: Option<String>,
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: String,
}
