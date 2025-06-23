use common_enums::enums;
use common_utils::{
    events::{ApiEventMetric, ApiEventsType},
    id_type,
};
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;
use utoipa::ToSchema;

use crate::payments::CustomerDetails;

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

    /// The currency for the transaction, required.
    #[schema(value_type = Currency)]
    pub currency: common_enums::Currency,

    /// The URL to which the user should be redirected after authentication.
    #[schema(value_type = Option<String>, example = "https://example.com/redirect")]
    pub return_url: Option<String>,

    /// Acquirer details information
    #[schema(value_type = Option<AcquirerDetails>)]
    pub acquirer_details: Option<AcquirerDetails>,

    /// Metadata for the authentication.
    #[schema(value_type = Option<Object>, example = json!({"order_id": "OR_12345"}))]
    pub metadata: Option<serde_json::Value>,

    /// Force 3DS challenge.
    #[serde(default)]
    pub force_3ds_challenge: Option<bool>,

    /// Choose what kind of sca exemption is required for this payment
    #[schema(value_type = Option<ScaExemptionType>)]
    pub psd2_sca_exemption_type: Option<common_enums::ScaExemptionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AcquirerDetails {
    pub bin: Option<String>,
    pub merchant_id: Option<String>,
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

    /// Customer details, if provided in the request.
    #[schema(value_type = Option<CustomerDetails>)]
    pub customer: Option<CustomerDetails>,

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

    /// You can specify up to 50 keys, with key names up to 40 characters long and values up to 500 characters long. Metadata is useful for storing additional, structured information on an object.
    #[schema(value_type = Option<Object>, example = r#"{ "udf1": "some-value", "udf2": "some-value" }"#)]
    pub metadata: Option<serde_json::Value>,

    /// The business profile that is associated with this payment
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,

    #[schema(value_type = Option<BrowserInformation>)]
    /// The browser information used for this payment
    pub browser_info: Option<serde_json::Value>,

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AuthenticationEligibilityRequest {
    /// Payment method data
    pub payment_method_data: crate::payments::PaymentMethodData,

    /// Payment method
    pub payment_method: common_enums::PaymentMethod,

    pub client_secret: Option<masking::Secret<String>>,

    /// The business profile that is associated with this payment
    #[schema(value_type = Option<String>)]
    pub profile_id: Option<id_type::ProfileId>,
}

impl AuthenticationEligibilityRequest {
    pub fn get_next_action_api(self, base_url: String, authentication_id: String) -> String {
        format!("{base_url}/authentication/{authentication_id}/authenticate")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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
