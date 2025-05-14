use api_models::payments::ThreeDsCompletionIndicator;
use cards::CardNumber;
use common_utils::types;
use masking::{Deserialize, Secret, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GpaymentsConnectorMetaData {
    pub authentication_url: String,
    pub three_ds_requestor_trans_id: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpaymentsPreAuthVersionCallRequest {
    pub acct_number: CardNumber,
    pub merchant_id: common_utils::id_type::MerchantId,
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
    /// Always returns 'Error' to indicate that this message is an error.
    ///
    /// Example: "Error"
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
    pub merchant_id: common_utils::id_type::MerchantId,

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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpaymentsAuthenticationRequest {
    pub acct_number: CardNumber,
    pub authentication_ind: String,
    pub browser_info_collected: BrowserInfoCollected,
    pub card_expiry_date: String,
    #[serde(rename = "notificationURL")]
    pub notification_url: String,
    pub merchant_id: common_utils::id_type::MerchantId,
    #[serde(rename = "threeDSCompInd")]
    pub three_ds_comp_ind: ThreeDsCompletionIndicator,
    pub message_category: String,
    pub purchase_amount: String,
    pub purchase_date: String,
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: String,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BrowserInfoCollected {
    pub browser_accept_header: Option<String>,
    pub browser_color_depth: Option<String>,
    #[serde(rename = "browserIP")]
    pub browser_ip: Option<Secret<String, common_utils::pii::IpAddress>>,
    pub browser_javascript_enabled: Option<bool>,
    pub browser_java_enabled: Option<bool>,
    pub browser_language: Option<String>,
    pub browser_screen_height: Option<String>,
    pub browser_screen_width: Option<String>,
    #[serde(rename = "browserTZ")]
    pub browser_tz: Option<String>,
    pub browser_user_agent: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum AuthenticationInd {
    #[serde(rename = "01")]
    PaymentTransaction,
    #[serde(rename = "02")]
    RecurringTransaction,
    #[serde(rename = "03")]
    InstalmentTransaction,
    #[serde(rename = "04")]
    AddCard,
    #[serde(rename = "05")]
    MaintainCard,
    #[serde(rename = "06")]
    CardholderVerification,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct GpaymentsAuthenticationSuccessResponse {
    #[serde(rename = "dsReferenceNumber")]
    pub ds_reference_number: String,
    #[serde(rename = "dsTransID")]
    pub ds_trans_id: String,
    #[serde(rename = "threeDSServerTransID")]
    pub three_ds_server_trans_id: String,
    #[serde(rename = "messageVersion")]
    pub message_version: String,
    #[serde(rename = "transStatus")]
    pub trans_status: AuthStatus,
    #[serde(rename = "acsTransID")]
    pub acs_trans_id: String,
    #[serde(rename = "challengeUrl")]
    pub acs_url: Option<url::Url>,
    #[serde(rename = "acsReferenceNumber")]
    pub acs_reference_number: String,
    pub authentication_value: Option<String>,
}

#[derive(Deserialize, Debug, Clone, Serialize, PartialEq)]
pub enum AuthStatus {
    /// Authentication/ Account Verification Successful
    Y,
    /// Not Authenticated /Account Not Verified; Transaction denied
    N,
    /// Authentication/ Account Verification Could Not Be Performed; Technical or other problem, as indicated in ARes or RReq
    U,
    /// Attempts Processing Performed; Not Authenticated/Verified , but a proof of attempted authentication/verification is provided
    A,
    /// Authentication/ Account Verification Rejected; Issuer is rejecting authentication/verification and request that authorisation not be attempted.
    R,
    /// Challenge required
    C,
}

impl From<AuthStatus> for common_enums::TransactionStatus {
    fn from(value: AuthStatus) -> Self {
        match value {
            AuthStatus::Y => Self::Success,
            AuthStatus::N => Self::Failure,
            AuthStatus::U => Self::VerificationNotPerformed,
            AuthStatus::A => Self::NotVerified,
            AuthStatus::R => Self::Rejected,
            AuthStatus::C => Self::ChallengeRequired,
        }
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GpaymentsPostAuthenticationResponse {
    pub authentication_value: Option<String>,
    pub trans_status: AuthStatus,
    pub eci: Option<String>,
}
