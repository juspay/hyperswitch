use common_enums::{enums, AuthenticationConnectors};
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
