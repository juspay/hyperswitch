use common_enums::{enums, AuthenticationConnectors};
#[cfg(feature = "v1")]
use common_utils::errors::{self, CustomResult};
use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    id_type,
};
#[cfg(feature = "v1")]
use error_stack::ResultExt;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

#[cfg(feature = "v1")]
use crate::payments::{Address, BrowserInformation, PaymentMethodData};
use crate::payments::{CustomerDetails, DeviceChannel, SdkInformation, ThreeDsCompletionIndicator};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationCreateRequest {
    /// The unique identifier for this authentication.
    #[schema(value_type = Option<String>, example = "auth_mbabizu24mvu3mela5njyhpit4")]
    pub authentication_id: Option<id_type::AuthenticationId>,

    /// The business profile that is associated with this authentication
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,

    /// Customer details.
    #[schema(value_type = Option<CustomerDetails>)]
    pub customer: Option<CustomerDetails>,

    /// The amount for the transaction, required.
    #[schema(value_type = MinorUnit, example = 1000)]
    pub amount: common_utils::types::MinorUnit,

    /// The connector to be used for authentication, if known.
    #[schema(value_type = Option<AuthenticationConnectors>, example = "netcetera")]
    pub authentication_connector: Option<AuthenticationConnectors>,

    /// The currency for the transaction, required.
    #[schema(value_type = Currency)]
    pub currency: common_enums::Currency,

    /// The URL to which the user should be redirected after authentication.
    #[schema(value_type = Option<String>, example = "https://example.com/redirect")]
    pub return_url: Option<String>,

    /// Force 3DS challenge.
    #[serde(default)]
    pub force_3ds_challenge: Option<bool>,

    /// Choose what kind of sca exemption is required for this payment
    #[schema(value_type = Option<ScaExemptionType>)]
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,

    /// Profile Acquirer ID get from profile acquirer configuration
    #[schema(value_type = Option<String>)]
    pub profile_acquirer_id: Option<id_type::ProfileAcquirerId>,

    /// Acquirer details information
    #[schema(value_type = Option<AcquirerDetails>)]
    pub acquirer_details: Option<AcquirerDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcquirerDetails {
    /// The bin of the card.
    #[schema(value_type = Option<String>, example = "123456")]
    pub acquirer_bin: Option<String>,
    /// The merchant id of the card.
    #[schema(value_type = Option<String>, example = "merchant_abc")]
    pub acquirer_merchant_id: Option<String>,
    /// The country code of the card.
    #[schema(value_type = Option<String>, example = "US/34456")]
    pub merchant_country_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationResponse {
    /// The unique identifier for this authentication.
    #[schema(value_type = String, example = "auth_mbabizu24mvu3mela5njyhpit4")]
    pub authentication_id: id_type::AuthenticationId,

    /// This is an identifier for the merchant account. This is inferred from the API key
    /// provided during the request
    #[schema(value_type = String, example = "merchant_abc")]
    pub merchant_id: id_type::MerchantId,

    /// The current status of the authentication (e.g., Started).
    #[schema(value_type = AuthenticationStatus)]
    pub status: common_enums::AuthenticationStatus,

    /// The client secret for this authentication, to be used for client-side operations.
    #[schema(value_type = Option<String>, example = "auth_mbabizu24mvu3mela5njyhpit4_secret_el9ksDkiB8hi6j9N78yo")]
    pub client_secret: Option<masking::Secret<String>>,

    /// The amount for the transaction.
    #[schema(value_type = MinorUnit, example = 1000)]
    pub amount: common_utils::types::MinorUnit,

    /// The currency for the transaction.
    #[schema(value_type = Currency)]
    pub currency: enums::Currency,

    /// The connector to be used for authentication, if known.
    #[schema(value_type = Option<AuthenticationConnectors>, example = "netcetera")]
    pub authentication_connector: Option<AuthenticationConnectors>,

    /// Whether 3DS challenge was forced.
    pub force_3ds_challenge: Option<bool>,

    /// The URL to which the user should be redirected after authentication, if provided.
    pub return_url: Option<String>,

    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601::option")]
    pub created_at: Option<PrimitiveDateTime>,

    #[schema(example = "E0001")]
    pub error_code: Option<String>,

    /// If there was an error while calling the connector the error message is received here
    #[schema(example = "Failed while verifying the card")]
    pub error_message: Option<String>,

    /// The business profile that is associated with this payment
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,

    /// Choose what kind of sca exemption is required for this payment
    #[schema(value_type = Option<ScaExemptionType>)]
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,

    /// Acquirer details information
    #[schema(value_type = Option<AcquirerDetails>)]
    pub acquirer_details: Option<AcquirerDetails>,

    /// Profile Acquirer ID get from profile acquirer configuration
    #[schema(value_type = Option<String>)]
    pub profile_acquirer_id: Option<id_type::ProfileAcquirerId>,
}

impl ApiEventMetric for AuthenticationCreateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        self.authentication_id
            .as_ref()
            .map(|id| ApiEventsType::Authentication {
                authentication_id: id.clone(),
            })
    }
}

impl ApiEventMetric for AuthenticationResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Authentication {
            authentication_id: self.authentication_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationEligibilityRequest {
    /// Payment method-specific data such as card details, wallet info, etc.
    /// This holds the raw information required to process the payment method.
    #[schema(value_type = PaymentMethodData)]
    pub payment_method_data: PaymentMethodData,

    /// Enum representing the type of payment method being used
    /// (e.g., Card, Wallet, UPI, BankTransfer, etc.).
    #[schema(value_type = PaymentMethod)]
    pub payment_method: common_enums::PaymentMethod,

    /// Optional secret value used to identify and authorize the client making the request.
    /// This can help ensure that the payment session is secure and valid.
    #[schema(value_type = Option<String>)]
    pub client_secret: Option<masking::Secret<String>>,

    /// Optional identifier for the business profile associated with the payment.
    /// This determines which configurations, rules, and branding are applied to the transaction.
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,

    /// Optional billing address of the customer.
    /// This can be used for fraud detection, authentication, or compliance purposes.
    #[schema(value_type = Option<Address>)]
    pub billing: Option<Address>,

    /// Optional shipping address of the customer.
    /// This can be useful for logistics, verification, or additional risk checks.
    #[schema(value_type = Option<Address>)]
    pub shipping: Option<Address>,

    /// Optional information about the customer's browser (user-agent, language, etc.).
    /// This is typically used to support 3DS authentication flows and improve risk assessment.
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_information: Option<BrowserInformation>,

    /// Optional email address of the customer.
    /// Used for customer identification, communication, and possibly for 3DS or fraud checks.
    #[schema(value_type = Option<String>)]
    pub email: Option<common_utils::pii::Email>,
}

#[cfg(feature = "v1")]
impl AuthenticationEligibilityRequest {
    pub fn get_next_action_api(
        &self,
        base_url: String,
        authentication_id: String,
    ) -> CustomResult<NextAction, errors::ParsingError> {
        let url = format!("{base_url}/authentication/{authentication_id}/authenticate");
        Ok(NextAction {
            url: url::Url::parse(&url).change_context(errors::ParsingError::UrlParsingError)?,
            http_method: common_utils::request::Method::Post,
        })
    }

    pub fn get_billing_address(&self) -> Option<Address> {
        self.billing.clone()
    }

    pub fn get_shipping_address(&self) -> Option<Address> {
        self.shipping.clone()
    }

    pub fn get_browser_information(&self) -> Option<BrowserInformation> {
        self.browser_information.clone()
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, Serialize, ToSchema)]
pub struct AuthenticationEligibilityResponse {
    /// The unique identifier for this authentication.
    #[schema(value_type = String, example = "auth_mbabizu24mvu3mela5njyhpit4")]
    pub authentication_id: id_type::AuthenticationId,
    /// The URL to which the user should be redirected after authentication.
    #[schema(value_type = NextAction)]
    pub next_action: NextAction,
    /// The current status of the authentication (e.g., Started).
    #[schema(value_type = AuthenticationStatus)]
    pub status: common_enums::AuthenticationStatus,
    /// The 3DS data for this authentication.
    #[schema(value_type = Option<EligibilityResponseParams>)]
    pub eligibility_response_params: Option<EligibilityResponseParams>,
    /// The metadata for this authentication.
    #[schema(value_type = serde_json::Value)]
    pub connector_metadata: Option<serde_json::Value>,
    /// The unique identifier for this authentication.
    #[schema(value_type = String)]
    pub profile_id: id_type::ProfileId,
    /// The error message for this authentication.
    #[schema(value_type = Option<String>)]
    pub error_message: Option<String>,
    /// The error code for this authentication.
    #[schema(value_type = Option<String>)]
    pub error_code: Option<String>,
    /// The connector used for this authentication.
    #[schema(value_type = Option<AuthenticationConnectors>)]
    pub authentication_connector: Option<AuthenticationConnectors>,
    /// Billing address
    #[schema(value_type = Option<Address>)]
    pub billing: Option<Address>,
    /// Shipping address
    #[schema(value_type = Option<Address>)]
    pub shipping: Option<Address>,
    /// Browser information
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_information: Option<BrowserInformation>,
    /// Email
    #[schema(value_type = Option<String>)]
    pub email: common_utils::crypto::OptionalEncryptableEmail,
    /// Acquirer details information.
    #[schema(value_type = Option<AcquirerDetails>)]
    pub acquirer_details: Option<AcquirerDetails>,
}

#[derive(Debug, Serialize, ToSchema)]
pub enum EligibilityResponseParams {
    ThreeDsData(ThreeDsData),
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ThreeDsData {
    /// The unique identifier for this authentication from the 3DS server.
    #[schema(value_type = String)]
    pub three_ds_server_transaction_id: Option<String>,
    /// The maximum supported 3DS version.
    #[schema(value_type = String)]
    pub maximum_supported_3ds_version: Option<common_utils::types::SemanticVersion>,
    /// The unique identifier for this authentication from the connector.
    #[schema(value_type = String)]
    pub connector_authentication_id: Option<String>,
    /// The data required to perform the 3DS method.
    #[schema(value_type = String)]
    pub three_ds_method_data: Option<String>,
    /// The URL to which the user should be redirected after authentication.
    #[schema(value_type = String, example = "https://example.com/redirect")]
    pub three_ds_method_url: Option<url::Url>,
    /// The version of the message.
    #[schema(value_type = String)]
    pub message_version: Option<common_utils::types::SemanticVersion>,
    /// The unique identifier for this authentication.
    #[schema(value_type = String)]
    pub directory_server_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NextAction {
    /// The URL for authenticatating the user.
    #[schema(value_type = String)]
    pub url: url::Url,
    /// The HTTP method to use for the request.
    #[schema(value_type = Method)]
    pub http_method: common_utils::request::Method,
}

#[cfg(feature = "v1")]
impl ApiEventMetric for AuthenticationEligibilityRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[cfg(feature = "v1")]
impl ApiEventMetric for AuthenticationEligibilityResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Authentication {
            authentication_id: self.authentication_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationAuthenticateRequest {
    /// Authentication ID for the authentication
    #[serde(skip_deserializing)]
    pub authentication_id: id_type::AuthenticationId,
    /// Client secret for the authentication
    #[schema(value_type = String)]
    pub client_secret: Option<masking::Secret<String>>,
    /// SDK Information if request is from SDK
    pub sdk_information: Option<SdkInformation>,
    /// Device Channel indicating whether request is coming from App or Browser
    pub device_channel: DeviceChannel,
    /// Indicates if 3DS method data was successfully completed or not
    pub threeds_method_comp_ind: ThreeDsCompletionIndicator,
}

impl ApiEventMetric for AuthenticationAuthenticateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Authentication {
            authentication_id: self.authentication_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationAuthenticateResponse {
    /// Indicates the transaction status
    #[serde(rename = "trans_status")]
    #[schema(value_type = Option<TransactionStatus>)]
    pub transaction_status: Option<common_enums::TransactionStatus>,
    /// Access Server URL to be used for challenge submission
    pub acs_url: Option<url::Url>,
    /// Challenge request which should be sent to acs_url
    pub challenge_request: Option<String>,
    /// Unique identifier assigned by the EMVCo(Europay, Mastercard and Visa)
    pub acs_reference_number: Option<String>,
    /// Unique identifier assigned by the ACS to identify a single transaction
    pub acs_trans_id: Option<String>,
    /// Unique identifier assigned by the 3DS Server to identify a single transaction
    pub three_ds_server_transaction_id: Option<String>,
    /// Contains the JWS object created by the ACS for the ARes(Authentication Response) message
    pub acs_signed_content: Option<String>,
    /// Three DS Requestor URL
    pub three_ds_requestor_url: String,
    /// Merchant app declaring their URL within the CReq message so that the Authentication app can call the Merchant app after OOB authentication has occurred
    pub three_ds_requestor_app_url: Option<String>,

    /// The error message for this authentication.
    #[schema(value_type = String)]
    pub error_message: Option<String>,
    /// The error code for this authentication.
    #[schema(value_type = String)]
    pub error_code: Option<String>,
    /// The authentication value for this authentication, only available in case of server to server request. Unavailable in case of client request due to security concern.
    #[schema(value_type = String)]
    pub authentication_value: Option<masking::Secret<String>>,

    /// The current status of the authentication (e.g., Started).
    #[schema(value_type = AuthenticationStatus)]
    pub status: common_enums::AuthenticationStatus,

    /// The connector to be used for authentication, if known.
    #[schema(value_type = Option<AuthenticationConnectors>, example = "netcetera")]
    pub authentication_connector: Option<AuthenticationConnectors>,

    /// The unique identifier for this authentication.
    #[schema(value_type = AuthenticationId, example = "auth_mbabizu24mvu3mela5njyhpit4")]
    pub authentication_id: id_type::AuthenticationId,

    /// The ECI value for this authentication.
    #[schema(value_type = String)]
    pub eci: Option<String>,

    /// Acquirer details information.
    #[schema(value_type = Option<AcquirerDetails>)]
    pub acquirer_details: Option<AcquirerDetails>,
}

impl ApiEventMetric for AuthenticationAuthenticateResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Authentication {
            authentication_id: self.authentication_id.clone(),
        })
    }
}

#[cfg(feature = "v1")]
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct AuthenticationSyncResponse {
    // Core Authentication Fields (from AuthenticationResponse)
    /// The unique identifier for this authentication.
    #[schema(value_type = String, example = "auth_mbabizu24mvu3mela5njyhpit4")]
    pub authentication_id: id_type::AuthenticationId,

    /// This is an identifier for the merchant account.
    #[schema(value_type = String, example = "merchant_abc")]
    pub merchant_id: id_type::MerchantId,

    /// The current status of the authentication.
    #[schema(value_type = AuthenticationStatus)]
    pub status: common_enums::AuthenticationStatus,

    /// The client secret for this authentication.
    #[schema(value_type = Option<String>)]
    pub client_secret: Option<masking::Secret<String>>,

    /// The amount for the transaction.
    #[schema(value_type = MinorUnit, example = 1000)]
    pub amount: common_utils::types::MinorUnit,

    /// The currency for the transaction.
    #[schema(value_type = Currency)]
    pub currency: enums::Currency,

    /// The connector used for authentication.
    #[schema(value_type = Option<AuthenticationConnectors>)]
    pub authentication_connector: Option<AuthenticationConnectors>,

    /// Whether 3DS challenge was forced.
    pub force_3ds_challenge: Option<bool>,

    /// The URL to which the user should be redirected after authentication.
    pub return_url: Option<String>,

    #[schema(example = "2022-09-10T10:11:12Z")]
    #[serde(with = "common_utils::custom_serde::iso8601")]
    pub created_at: PrimitiveDateTime,

    /// The business profile that is associated with this authentication.
    #[schema(value_type = String)]
    pub profile_id: id_type::ProfileId,

    /// SCA exemption type for this authentication.
    #[schema(value_type = Option<ScaExemptionType>)]
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,

    /// Acquirer details information.
    #[schema(value_type = Option<AcquirerDetails>)]
    pub acquirer_details: Option<AcquirerDetails>,

    /// The unique identifier from the 3DS server.
    #[schema(value_type = Option<String>)]
    pub threeds_server_transaction_id: Option<String>,

    /// The maximum supported 3DS version.
    #[schema(value_type = Option<String>)]
    pub maximum_supported_3ds_version: Option<common_utils::types::SemanticVersion>,

    /// The unique identifier from the connector.
    #[schema(value_type = Option<String>)]
    pub connector_authentication_id: Option<String>,

    /// The data required to perform the 3DS method.
    #[schema(value_type = Option<String>)]
    pub three_ds_method_data: Option<String>,

    /// The URL for the 3DS method.
    #[schema(value_type = Option<String>)]
    pub three_ds_method_url: Option<String>,

    /// The version of the message.
    #[schema(value_type = Option<String>)]
    pub message_version: Option<common_utils::types::SemanticVersion>,

    /// The metadata for this authentication.
    #[schema(value_type = Option<serde_json::Value>)]
    pub connector_metadata: Option<serde_json::Value>,

    /// The unique identifier for the directory server.
    #[schema(value_type = Option<String>)]
    pub directory_server_id: Option<String>,

    /// Billing address.
    #[schema(value_type = Option<Address>)]
    pub billing: Option<Address>,

    /// Shipping address.
    #[schema(value_type = Option<Address>)]
    pub shipping: Option<Address>,

    /// Browser information.
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_information: Option<BrowserInformation>,

    /// Email.
    #[schema(value_type = Option<String>)]
    pub email: common_utils::crypto::OptionalEncryptableEmail,

    /// Indicates the transaction status.
    #[serde(rename = "trans_status")]
    #[schema(value_type = Option<TransactionStatus>)]
    pub transaction_status: Option<common_enums::TransactionStatus>,

    /// Access Server URL for challenge submission.
    pub acs_url: Option<String>,

    /// Challenge request to be sent to acs_url.
    pub challenge_request: Option<String>,

    /// Unique identifier assigned by EMVCo.
    pub acs_reference_number: Option<String>,

    /// Unique identifier assigned by the ACS.
    pub acs_trans_id: Option<String>,

    /// JWS object created by the ACS for the ARes message.
    pub acs_signed_content: Option<String>,

    /// Three DS Requestor URL.
    pub three_ds_requestor_url: Option<String>,

    /// Merchant app URL for OOB authentication.
    pub three_ds_requestor_app_url: Option<String>,

    /// The authentication value for this authentication, only available in case of server to server request. Unavailable in case of client request due to security concern.
    #[schema(value_type = Option<String>)]
    pub authentication_value: Option<masking::Secret<String>>,

    /// ECI value for this authentication, only available in case of server to server request. Unavailable in case of client request due to security concern.
    pub eci: Option<String>,

    // Common Error Fields (present in multiple responses)
    /// Error message if any.
    #[schema(value_type = Option<String>)]
    pub error_message: Option<String>,

    /// Error code if any.
    #[schema(value_type = Option<String>)]
    pub error_code: Option<String>,

    /// Profile Acquirer ID
    #[schema(value_type = Option<String>)]
    pub profile_acquirer_id: Option<id_type::ProfileAcquirerId>,
}

#[cfg(feature = "v1")]
impl ApiEventMetric for AuthenticationSyncResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Authentication {
            authentication_id: self.authentication_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationSyncRequest {
    /// The client secret for this authentication.
    #[schema(value_type = String)]
    pub client_secret: Option<masking::Secret<String>>,
    /// Authentication ID for the authentication
    #[serde(skip_deserializing)]
    pub authentication_id: id_type::AuthenticationId,
}

impl ApiEventMetric for AuthenticationSyncRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Authentication {
            authentication_id: self.authentication_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationSyncPostUpdateRequest {
    /// Authentication ID for the authentication
    #[serde(skip_deserializing)]
    pub authentication_id: id_type::AuthenticationId,
}

impl ApiEventMetric for AuthenticationSyncPostUpdateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Authentication {
            authentication_id: self.authentication_id.clone(),
        })
    }
}
