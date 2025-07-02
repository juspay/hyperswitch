use common_enums::{enums, AuthenticationConnectors};
use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    id_type, pii,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::payments::{Address, BrowserInformation, CustomerDetails};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationCreateRequest {
    /// The unique identifier for this authentication.
    #[schema(value_type = Option<String>, example = "auth_mbabizu24mvu3mela5njyhpit4")]
    pub authentication_id: Option<id_type::AuthenticationId>,

    /// The business profile that is associated with this authentication
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,

    /// The connector to be used for authentication, if known.
    #[schema(value_type = Option<AuthenticationConnectors>, example = "netcetera")]
    pub authentication_connector: Option<AuthenticationConnectors>,

    /// Customer details.
    #[schema(value_type = Option<CustomerDetails>)]
    pub customer: Option<CustomerDetails>,

    /// The amount for the transaction, required.
    #[schema(value_type = MinorUnit, example = 1000)]
    pub amount: common_utils::types::MinorUnit,

    /// The currency for the transaction, required.
    #[schema(value_type = Currency)]
    pub currency: common_enums::Currency,

    /// The URL to which the user should be redirected after authentication.
    #[schema(value_type = Option<String>, example = "https://example.com/redirect")]
    pub return_url: Option<String>,

    /// Acquirer details information
    #[schema(value_type = Option<AcquirerDetails>)]
    pub acquirer_details: Option<AcquirerDetails>,

    /// Force 3DS challenge.
    #[serde(default)]
    pub force_3ds_challenge: Option<bool>,

    /// Choose what kind of sca exemption is required for this payment
    #[schema(value_type = Option<ScaExemptionType>)]
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcquirerDetails {
    /// The bin of the card.
    #[schema(value_type = Option<String>, example = "123456")]
    pub bin: Option<String>,
    /// The merchant id of the card.
    #[schema(value_type = Option<String>, example = "merchant_abc")]
    pub merchant_id: Option<String>,
    /// The country code of the card.
    #[schema(value_type = Option<String>, example = "US/34456")]
    pub country_code: Option<String>,
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

    /// Whether 3DS challenge was forced.
    pub force_3ds_challenge: Option<bool>,

    /// The connector to be used for authentication, if specified in request.
    #[schema(value_type = Option<AuthenticationConnectors>)]
    pub authentication_connector: Option<String>,

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

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationEligibilityRequest {
    pub payment_method_data: crate::payments::PaymentMethodData,

    /// Payment method
    pub payment_method: common_enums::PaymentMethod,

    pub client_secret: Option<masking::Secret<String>>,

    /// The business profile that is associated with this payment
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,

    /// Billing address
    #[schema(value_type = Option<Address>)]
    pub billing: Option<Address>,

    /// Shipping address
    #[schema(value_type = Option<Address>)]
    pub shipping: Option<Address>,

    /// Browser information
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_information: Option<BrowserInformation>,

    pub email: Option<pii::Email>,
}

impl AuthenticationEligibilityRequest {
    pub fn get_next_action_api(&self, base_url: String, authentication_id: String) -> String {
        format!("{base_url}/authentication/{authentication_id}/authenticate")
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

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthenticationEligibilityResponse {
    pub authentication_id: id_type::AuthenticationId,
    pub next_api_action: String,
    /// The current status of the authentication (e.g., Started).
    #[schema(value_type = AuthenticationStatus)]
    pub status: common_enums::AuthenticationStatus,
    pub threeds_server_transaction_id: Option<String>,
    pub maximum_supported_3ds_version: Option<common_utils::types::SemanticVersion>,
    pub connector_authentication_id: Option<String>,
    pub three_ds_method_data: Option<String>,
    pub three_ds_method_url: Option<String>,
    pub message_version: Option<common_utils::types::SemanticVersion>,
    pub connector_metadata: Option<serde_json::Value>,
    pub directory_server_id: Option<String>,
    pub profile_id: id_type::ProfileId,
    pub error_message: Option<String>,
    pub error_code: Option<String>,
    pub authentication_connector: Option<String>,
    /// Billing address
    #[schema(value_type = Option<Address>)]
    pub billing: Option<Address>,

    /// Shipping address
    #[schema(value_type = Option<Address>)]
    pub shipping: Option<Address>,

    /// Browser information
    #[schema(value_type = Option<BrowserInformation>)]
    pub browser_information: Option<BrowserInformation>,

    pub email: common_utils::crypto::OptionalEncryptableEmail,
}

impl ApiEventMetric for AuthenticationEligibilityRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

impl ApiEventMetric for AuthenticationEligibilityResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        Some(ApiEventsType::Authentication {
            authentication_id: self.authentication_id.clone(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationAuthenticateRequest {
    #[schema(value_type = String)]
    pub client_secret: masking::Secret<String>,
    /// SDK Information if request is from SDK
    pub sdk_information: Option<crate::payments::SdkInformation>,
    /// Device Channel indicating whether request is coming from App or Browser
    pub device_channel: crate::payments::DeviceChannel,
    /// Indicates if 3DS method data was successfully completed or not
    pub threeds_method_comp_ind: crate::payments::ThreeDsCompletionIndicator,
}

impl ApiEventMetric for AuthenticationAuthenticateRequest {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationAuthenticateResponse {
    /// Indicates the transaction status
    #[serde(rename = "trans_status")]
    #[schema(value_type = TransactionStatus)]
    pub transaction_status: common_enums::TransactionStatus,
    /// Access Server URL to be used for challenge submission
    pub acs_url: Option<String>,
    /// Challenge request which should be sent to acs_url
    pub challenge_request: Option<String>,
    /// Unique identifier assigned by the EMVCo(Europay, Mastercard and Visa)
    pub acs_reference_number: Option<String>,
    /// Unique identifier assigned by the ACS to identify a single transaction
    pub acs_trans_id: Option<String>,
    /// Unique identifier assigned by the 3DS Server to identify a single transaction
    pub three_dsserver_trans_id: Option<String>,
    /// Contains the JWS object created by the ACS for the ARes(Authentication Response) message
    pub acs_signed_content: Option<String>,
    /// Three DS Requestor URL
    pub three_ds_requestor_url: String,
    /// Merchant app declaring their URL within the CReq message so that the Authentication app can call the Merchant app after OOB authentication has occurred
    pub three_ds_requestor_app_url: Option<String>,
}

impl ApiEventMetric for AuthenticationAuthenticateResponse {
    fn get_api_event_type(&self) -> Option<ApiEventsType> {
        None
    }
}
