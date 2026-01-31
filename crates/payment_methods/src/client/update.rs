//! Update payment method flow types and modular models.

use cards::CardNumber;
use common_enums::{
    CardNetwork, ConnectorTokenStatus, CountryAlpha2, Currency, PaymentMethod, PaymentMethodType,
    StorageType, TokenizationType,
};
use common_utils::{
    id_type,
    pii::SecretSerdeValue,
    request::{Method, RequestContent},
    types::MinorUnit,
};
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

/// V1-facing update flow type.
#[derive(Debug)]
pub struct UpdatePaymentMethod;

/// V1-facing update request payload.
#[derive(Debug)]
pub struct UpdatePaymentMethodV1Request {
    /// Identifier for the payment method to update.
    pub payment_method_id: String,
    /// Typed update payload derived from aggregated data.
    pub payload: UpdatePaymentMethodV1Payload,
}

/// V1-facing update payload.
#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePaymentMethodV1Payload {
    /// Payment method details to update.
    pub payment_method_data: Option<PaymentMethodUpdateData>,
    /// Connector token details to update.
    pub connector_token_details: Option<ConnectorTokenDetails>,
    /// Network transaction ID for off-session updates.
    pub network_transaction_id: Option<Secret<String>>,
}

/// Modular service update request payload.
#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePaymentMethodV2Request {
    /// Payment method details to update.
    pub payment_method_data: Option<PaymentMethodUpdateData>,
    /// Connector token details to update.
    pub connector_token_details: Option<ConnectorTokenDetails>,
    /// Network transaction ID for off-session updates.
    pub network_transaction_id: Option<Secret<String>>,
}

/// Payment method update data.
#[derive(Debug, Serialize, Clone)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "payment_method_data")]
pub enum PaymentMethodUpdateData {
    Card(CardDetailUpdate),
}

/// Card update payload for the modular service.
#[derive(Debug, Serialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct CardDetailUpdate {
    /// Card holder name.
    pub card_holder_name: Option<Secret<String>>,
    /// Card holder nickname.
    pub nick_name: Option<Secret<String>>,
    /// Card CVC (optional).
    pub card_cvc: Option<Secret<String>>,
}

/// Connector token details update payload.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConnectorTokenDetails {
    /// The connector account identifier.
    pub connector_id: id_type::MerchantConnectorAccountId,
    /// The tokenization type of the connector token.
    pub token_type: TokenizationType,
    /// Status of the connector token.
    pub status: ConnectorTokenStatus,
    /// Reference id used while creating the token at the connector.
    pub connector_token_request_reference_id: Option<String>,
    /// Original amount authorized for this token.
    pub original_payment_authorized_amount: Option<MinorUnit>,
    /// Currency of the original authorized amount.
    pub original_payment_authorized_currency: Option<Currency>,
    /// Metadata associated with the connector token.
    pub metadata: Option<SecretSerdeValue>,
    /// Connector token value.
    pub token: Secret<String>,
}

/// Modular service update response payload.
#[derive(Debug, Deserialize)]
pub struct UpdatePaymentMethodV2Response {
    /// The unique identifier of the payment method.
    pub id: String,
    /// Unique identifier for a merchant.
    pub merchant_id: id_type::MerchantId,
    /// The unique identifier of the customer.
    pub customer_id: Option<id_type::CustomerId>,
    /// The type of payment method.
    pub payment_method_type: Option<PaymentMethod>,
    /// The payment method subtype.
    pub payment_method_subtype: Option<PaymentMethodType>,
    /// Indicates whether recurring is enabled.
    pub recurring_enabled: Option<bool>,
    /// Timestamp for creation time.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
    /// Timestamp for last usage.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_used_at: Option<PrimitiveDateTime>,
    /// Payment method details.
    pub payment_method_data: Option<PaymentMethodResponseData>,
    /// Connector token details if available.
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    /// Network token details if available.
    pub network_token: Option<NetworkTokenResponse>,
    /// Storage type.
    pub storage_type: Option<StorageType>,
    /// Stored card CVC token details.
    pub card_cvc_token_storage: Option<CardCVCTokenStorageDetails>,
}

/// V1-facing update response.
#[derive(Debug, Deserialize)]
pub struct UpdatePaymentMethodResponse {
    /// The unique identifier of the payment method.
    pub id: String,
    /// Unique identifier for a merchant.
    pub merchant_id: id_type::MerchantId,
    /// The unique identifier of the customer.
    pub customer_id: Option<id_type::CustomerId>,
    /// The type of payment method.
    pub payment_method_type: Option<PaymentMethod>,
    /// The payment method subtype.
    pub payment_method_subtype: Option<PaymentMethodType>,
    /// Indicates whether recurring is enabled.
    pub recurring_enabled: Option<bool>,
    /// Timestamp for creation time.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub created: Option<PrimitiveDateTime>,
    /// Timestamp for last usage.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub last_used_at: Option<PrimitiveDateTime>,
    /// Payment method details.
    pub payment_method_data: Option<PaymentMethodResponseData>,
    /// Connector token details if available.
    pub connector_tokens: Option<Vec<ConnectorTokenDetails>>,
    /// Network token details if available.
    pub network_token: Option<NetworkTokenResponse>,
    /// Storage type.
    pub storage_type: Option<StorageType>,
    /// Stored card CVC token details.
    pub card_cvc_token_storage: Option<CardCVCTokenStorageDetails>,
}

/// Payment method response data.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "payment_method_data")]
pub enum PaymentMethodResponseData {
    Card(CardDetailFromLocker),
}

/// Card details as returned by the modular service.
#[derive(Debug, Deserialize)]
pub struct CardDetailFromLocker {
    pub issuer_country: Option<CountryAlpha2>,
    pub last4_digits: Option<String>,
    #[serde(skip)]
    pub card_number: Option<CardNumber>,
    pub expiry_month: Option<Secret<String>>,
    pub expiry_year: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_fingerprint: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub card_network: Option<CardNetwork>,
    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_type: Option<String>,
    pub saved_to_locker: bool,
}

fn saved_in_locker_default() -> bool {
    true
}

/// Network token response payload.
#[derive(Debug, Deserialize)]
pub struct NetworkTokenResponse {
    pub payment_method_data: NetworkTokenDetailsPaymentMethod,
}

/// Network token payment method details.
#[derive(Debug, Deserialize)]
pub struct NetworkTokenDetailsPaymentMethod {
    pub last4_digits: Option<String>,
    pub issuer_country: Option<CountryAlpha2>,
    pub network_token_expiry_month: Option<Secret<String>>,
    pub network_token_expiry_year: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub card_holder_name: Option<Secret<String>>,
    pub card_isin: Option<String>,
    pub card_issuer: Option<String>,
    pub card_network: Option<CardNetwork>,
    pub card_type: Option<String>,
    #[serde(default = "saved_in_locker_default")]
    pub saved_to_locker: bool,
}

/// Stored card CVC token details.
#[derive(Clone, Copy, Debug, PartialEq, Deserialize)]
pub struct CardCVCTokenStorageDetails {
    /// Whether card CVC is stored.
    pub is_stored: bool,
    /// Expiry timestamp for stored CVC.
    #[serde(default, with = "common_utils::custom_serde::iso8601::option")]
    pub expires_at: Option<PrimitiveDateTime>,
}

impl TryFrom<&UpdatePaymentMethodV1Request> for UpdatePaymentMethodV2Request {
    type Error = MicroserviceClientError;

    fn try_from(value: &UpdatePaymentMethodV1Request) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: value.payload.payment_method_data.clone(),
            connector_token_details: value.payload.connector_token_details.clone(),
            network_transaction_id: value.payload.network_transaction_id.clone(),
        })
    }
}

impl TryFrom<UpdatePaymentMethodV2Response> for UpdatePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(value: UpdatePaymentMethodV2Response) -> Result<Self, Self::Error> {
        Ok(Self {
            id: value.id,
            merchant_id: value.merchant_id,
            customer_id: value.customer_id,
            payment_method_type: value.payment_method_type,
            payment_method_subtype: value.payment_method_subtype,
            recurring_enabled: value.recurring_enabled,
            created: value.created,
            last_used_at: value.last_used_at,
            payment_method_data: value.payment_method_data,
            connector_tokens: value.connector_tokens,
            network_token: value.network_token,
            storage_type: value.storage_type,
            card_cvc_token_storage: value.card_cvc_token_storage,
        })
    }
}

impl UpdatePaymentMethod {
    fn validate_request(
        &self,
        request: &UpdatePaymentMethodV1Request,
    ) -> Result<(), MicroserviceClientError> {
        if request.payment_method_id.trim().is_empty() {
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
        request: &UpdatePaymentMethodV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![("id", request.payment_method_id.clone())]
    }

    fn build_body(&self, request: UpdatePaymentMethodV2Request) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(request)))
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    UpdatePaymentMethod,
    method = Method::Put,
    path = "/v2/payment-methods/{id}/update-saved-payment-method",
    v1_request = UpdatePaymentMethodV1Request,
    v2_request = UpdatePaymentMethodV2Request,
    v2_response = UpdatePaymentMethodV2Response,
    v1_response = UpdatePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    body = UpdatePaymentMethod::build_body,
    path_params = UpdatePaymentMethod::build_path_params,
    validate = UpdatePaymentMethod::validate_request
);
