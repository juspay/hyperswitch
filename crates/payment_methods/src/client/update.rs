//! Update payment method flow types and modular models.

use api_models::payment_methods::NetworkTokenResponse;
use common_enums::{PaymentMethod, PaymentMethodType, StorageType};
use common_utils::{
    id_type,
    request::{Method, RequestContent},
};
use hyperswitch_interfaces::micro_service::{MicroserviceClientError, MicroserviceClientErrorKind};
use masking::Secret;
use serde::{Deserialize, Serialize};
use time::PrimitiveDateTime;

use crate::types::{
    CardCVCTokenStorageDetails, ConnectorTokenDetails, ModularPMRetrieveResponse,
    PaymentMethodResponseData,
};

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
    pub modular_service_prefix: String,
}

/// V1-facing update payload.
#[derive(Clone, Debug, Serialize)]
pub struct UpdatePaymentMethodV1Payload {
    /// Payment method details to update.
    pub payment_method_data: Option<PaymentMethodUpdateData>,
    /// Connector token details to update.
    pub connector_token_details: Option<ConnectorTokenDetails>,
    /// Network transaction ID for off-session updates.
    pub network_transaction_id: Option<Secret<String>>,

    pub acknowledgement_status: Option<common_enums::AcknowledgementStatus>,
}

/// Modular service update request payload.
#[derive(Clone, Debug, Serialize)]
pub struct ModularPMUpdateRequest {
    /// Payment method details to update.
    pub payment_method_data: Option<PaymentMethodUpdateData>,
    /// Connector token details to update.
    pub connector_token_details: Option<ConnectorTokenDetails>,
    /// Network transaction ID for off-session updates.
    pub network_transaction_id: Option<Secret<String>>,

    pub acknowledgement_status: Option<common_enums::AcknowledgementStatus>,
}

/// Payment method update data.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "snake_case")]
#[serde(rename = "payment_method_data")]
pub enum PaymentMethodUpdateData {
    Card(CardDetailUpdate),
}

/// Card update payload for the modular service.
#[derive(Debug, Serialize, Clone)]
pub struct CardDetailUpdate {
    /// Card holder name.
    pub card_holder_name: Option<Secret<String>>,
    /// Card holder nickname.
    pub nick_name: Option<Secret<String>>,
    /// Card CVC (optional).
    pub card_cvc: Option<Secret<String>>,
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
    pub payment_method_type: PaymentMethod,
    /// The payment method subtype.
    pub payment_method_subtype: PaymentMethodType,
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

impl TryFrom<&UpdatePaymentMethodV1Request> for ModularPMUpdateRequest {
    type Error = MicroserviceClientError;

    fn try_from(value: &UpdatePaymentMethodV1Request) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_data: value.payload.payment_method_data.clone(),
            connector_token_details: value.payload.connector_token_details.clone(),
            network_transaction_id: value.payload.network_transaction_id.clone(),
            acknowledgement_status: value.payload.acknowledgement_status,
        })
    }
}

impl TryFrom<ModularPMRetrieveResponse> for UpdatePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(value: ModularPMRetrieveResponse) -> Result<Self, Self::Error> {
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
        common_utils::fp_utils::when(request.payment_method_id.trim().is_empty(), || {
            Err(MicroserviceClientError {
                operation: std::any::type_name::<Self>().to_string(),
                kind: MicroserviceClientErrorKind::InvalidRequest(
                    "Payment method ID cannot be empty".to_string(),
                ),
            })
        })?;
        Ok(())
    }

    fn build_path_params(
        &self,
        request: &UpdatePaymentMethodV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![
            ("prefix", request.modular_service_prefix.clone()),
            ("id", request.payment_method_id.clone()),
        ]
    }

    fn build_body(&self, request: ModularPMUpdateRequest) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(request)))
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    UpdatePaymentMethod,
    method = Method::Put,
    path = "/{prefix}/payment-methods/{id}/update-saved-payment-method",
    v1_request = UpdatePaymentMethodV1Request,
    v2_request = ModularPMUpdateRequest,
    v2_response = ModularPMRetrieveResponse,
    v1_response = UpdatePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    body = UpdatePaymentMethod::build_body,
    path_params = UpdatePaymentMethod::build_path_params,
    validate = UpdatePaymentMethod::validate_request
);
