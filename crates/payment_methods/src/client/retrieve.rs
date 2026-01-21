//! Retrieve payment method flow types and models.

use api_models::payment_methods::{CardDetailFromLocker, PaymentMethodId};
use cards::CardNumber;
use common_enums::{PaymentMethod, PaymentMethodType};
use common_utils::{id_type, pii, request::Method};
use hyperswitch_domain_models::payment_method_data::NetworkTokenDetailsPaymentMethod;
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use serde::Deserialize;
use time;

/// V1-facing retrieve flow input.
#[derive(Debug)]
pub struct RetrievePaymentMethod {
    /// Identifier for the payment method to fetch.
    pub payment_method_id: PaymentMethodId,
}

impl RetrievePaymentMethod {
    /// Construct a new retrieve flow.
    pub fn new(payment_method_id: PaymentMethodId) -> Self {
        Self { payment_method_id }
    }
}

/// V2 modular service request payload.
#[derive(Clone, Debug)]
pub struct RetrievePaymentMethodV2Request {
    /// Identifier for the payment method to fetch.
    pub payment_method_id: PaymentMethodId,
}

/// V2 PaymentMethodResponse as returned by the V2 API.
/// This is a copy of the V2 PaymentMethodResponse struct from api_models for use in V1-only builds.
#[derive(Clone, Debug, Deserialize)]
pub struct RetrievePaymentMethodV2Response {
    /// The unique identifier of the Payment method
    pub id: String,

    /// Unique identifier for a merchant
    pub merchant_id: id_type::MerchantId,

    /// The unique identifier of the customer.
    pub customer_id: Option<String>,

    /// The type of payment method use for the payment.
    pub payment_method_type: Option<PaymentMethod>,

    /// This is a sub-category of payment method.
    pub payment_method_subtype: Option<PaymentMethodType>,

    /// Indicates whether the payment method supports recurring payments. Optional.
    pub recurring_enabled: Option<bool>,

    /// A timestamp (ISO 8601 code) that determines when the payment method was created
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<time::PrimitiveDateTime>,

    /// A timestamp (ISO 8601 code) that determines when the payment method was last used
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_used_at: Option<time::PrimitiveDateTime>,

    /// The payment method details related to the payment method
    pub payment_method_data: Option<PaymentMethodResponseData>,

    /// The connector token details if available (ignored in V1 response)
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,

    /// Network token details if available (ignored in V1 response)
    pub network_token: Option<NetworkTokenResponse>,

    /// The storage type for the payment method (ignored in V1 response)
    pub storage_type: Option<common_enums::StorageType>,

    /// Card CVC token storage details (ignored in V1 response)
    pub card_cvc_token_storage: Option<CardCVCTokenStorageDetails>,
}

/// V2 ConnectorTokenDetails (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct ConnectorTokenDetails {
    /// The unique identifier of the connector account through which the token was generated
    pub connector_id: id_type::MerchantConnectorAccountId,
    /// The type of tokenization used
    pub token_type: common_enums::TokenizationType,
    /// The status of connector token if it is active or inactive
    pub status: common_enums::ConnectorTokenStatus,
    /// The reference id of the connector token
    /// This is the reference that was passed to connector when creating the token
    pub connector_token_request_reference_id: Option<String>,
    pub original_payment_authorized_amount: Option<common_utils::types::MinorUnit>,
    /// The currency of the original payment authorized amount
    pub original_payment_authorized_currency: Option<common_enums::Currency>,
    /// Metadata associated with the connector token
    pub metadata: Option<pii::SecretSerdeValue>,
    /// The value of the connector token. This token can be used to make merchant initiated payments ( MIT ), directly with the connector.
    pub token: masking::Secret<String>,
}

/// V2 NetworkTokenResponse (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct NetworkTokenResponse {
    pub payment_method_data: NetworkTokenDetailsPaymentMethod,
}

/// V2 CardCVCTokenStorageDetails (for deserialization, ignored in transformation)
#[derive(Clone, Debug, Deserialize)]
pub struct CardCVCTokenStorageDetails {
    pub is_stored: bool,
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expires_at: Option<time::PrimitiveDateTime>,
}

/// V2 PaymentMethodResponseData enum
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "payment_method_data")]
pub enum PaymentMethodResponseData {
    Card(CardDetailFromLockerV2),
}

/// V2 CardDetailFromLocker for deserialization
#[derive(Clone, Debug, Deserialize)]
pub struct CardDetailFromLockerV2 {
    pub issuer_country: Option<common_enums::CountryAlpha2>,
    pub last4_digits: Option<String>,
    #[serde(skip)]
    pub card_number: Option<CardNumber>,
    pub expiry_month: Option<masking::Secret<String>>,
    pub expiry_year: Option<masking::Secret<String>>,
    pub card_holder_name: Option<masking::Secret<String>>,
    pub card_fingerprint: Option<masking::Secret<String>>,
    pub nick_name: Option<masking::Secret<String>>,
    pub card_network: Option<common_enums::CardNetwork>,
    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_type: Option<String>,
    pub saved_to_locker: bool,
}

/// V1-facing retrieve response.
#[derive(Clone, Debug)]
pub struct RetrievePaymentMethodResponse {
    /// V1 payment method identifier.
    pub payment_method_id: String,
    /// Merchant ID.
    pub merchant_id: id_type::MerchantId,
    /// Customer ID.
    pub customer_id: Option<id_type::CustomerId>,
    /// Payment method type.
    pub payment_method: Option<PaymentMethod>,
    /// Payment method subtype.
    pub payment_method_type: Option<PaymentMethodType>,
    /// Card details.
    pub card: Option<CardDetailFromLocker>,
    /// Recurring enabled.
    pub recurring_enabled: Option<bool>,
    /// Created timestamp.
    pub created: Option<time::PrimitiveDateTime>,
    /// Last used timestamp.
    pub last_used_at: Option<time::PrimitiveDateTime>,
}

impl TryFrom<&RetrievePaymentMethod> for RetrievePaymentMethodV2Request {
    type Error = MicroserviceClientError;

    fn try_from(value: &RetrievePaymentMethod) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: value.payment_method_id.clone(),
        })
    }
}

impl TryFrom<RetrievePaymentMethodV2Response> for RetrievePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(v2_resp: RetrievePaymentMethodV2Response) -> Result<Self, Self::Error> {
        // Extract payment_method_id from GlobalPaymentMethodId
        let payment_method_id = v2_resp.id.clone();

        // Convert GlobalCustomerId to CustomerId
        let customer_id = v2_resp
            .customer_id
            .map(|id| id_type::CustomerId::try_from(std::borrow::Cow::from(id)))
            .transpose()
            .map_err(|e| MicroserviceClientError {
                operation: "convert_global_customer_id".to_string(),
                kind: MicroserviceClientErrorKind::Deserialize(format!(
                    "Failed to convert customer ID: {}",
                    e
                )),
            })?;

        // Convert card details from V2 to V1 format
        let card = v2_resp.payment_method_data.map(|pmd| match pmd {
            PaymentMethodResponseData::Card(v2_card) => CardDetailFromLocker {
                scheme: None, // V2 doesn't have this field
                issuer_country: v2_card.issuer_country.map(|c| c.to_string()),
                issuer_country_code: None, // V2 doesn't have this field
                last4_digits: v2_card.last4_digits,
                card_number: v2_card.card_number,
                expiry_month: v2_card.expiry_month,
                expiry_year: v2_card.expiry_year,
                card_token: None, // V2 doesn't have this field
                card_holder_name: v2_card.card_holder_name,
                card_fingerprint: v2_card.card_fingerprint,
                nick_name: v2_card.nick_name,
                card_network: v2_card.card_network,
                card_isin: v2_card.card_isin,
                card_issuer: v2_card.card_issuer,
                card_type: v2_card.card_type,
                saved_to_locker: v2_card.saved_to_locker,
            },
        });

        Ok(Self {
            payment_method_id,
            merchant_id: v2_resp.merchant_id,
            customer_id,
            payment_method: v2_resp.payment_method_type,
            payment_method_type: v2_resp.payment_method_subtype,
            card,
            recurring_enabled: v2_resp.recurring_enabled,
            created: v2_resp.created,
            last_used_at: v2_resp.last_used_at,
        })
    }
}

impl RetrievePaymentMethod {
    fn validate_request(&self) -> Result<(), MicroserviceClientError> {
        if self.payment_method_id.payment_method_id.trim().is_empty() {
            return Err(MicroserviceClientError {
                operation: std::any::type_name::<Self>().to_string(),
                kind: MicroserviceClientErrorKind::InvalidRequest(
                    "Payment method ID cannot be empty".to_string(),
                ),
            });
        }
        Ok(())
    }

    fn build_path_params(
        &self,
        request: &RetrievePaymentMethodV2Request,
    ) -> Vec<(&'static str, String)> {
        vec![("id", request.payment_method_id.payment_method_id.clone())]
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    RetrievePaymentMethod,
    method = Method::Get,
    path = "/v2/payment-methods/{id}",
    v2_request = RetrievePaymentMethodV2Request,
    v2_response = RetrievePaymentMethodV2Response,
    v1_response = RetrievePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    path_params = RetrievePaymentMethod::build_path_params,
    validate = RetrievePaymentMethod::validate_request
);
