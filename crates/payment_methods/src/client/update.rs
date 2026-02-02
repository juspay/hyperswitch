use common_utils::request::{Method, RequestContent};
use hyperswitch_interfaces::micro_service::MicroserviceClientError;
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::client::create::{ConnectorTokenDetails, ModularPaymentMethodResponse};

#[derive(Debug)]
pub struct UpdatePaymentMethod;

#[derive(Debug)]
pub struct UpdatePaymentMethodV1Request {
    /// Identifier for the payment method to update.
    /// Type String is used throughout v1 payment methods
    pub payment_method_id: String,
    pub payment_method_data: Option<PaymentMethodUpdateData>,
    pub connector_token_details: Option<ConnectorTokenDetails>,
    pub network_transaction_id: Option<Secret<String>>,
    pub modular_service_prefix: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModularPMUpdateRequest {
    pub payment_method_data: Option<PaymentMethodUpdateData>,
    pub connector_token_details: Option<ConnectorTokenDetails>,
    pub network_transaction_id: Option<Secret<String>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PaymentMethodUpdateData {
    Card(CardDetailUpdate),
}

#[derive(Debug, Clone, Serialize)]
pub struct CardDetailUpdate {
    pub card_holder_name: Option<Secret<String>>,
    pub nick_name: Option<Secret<String>>,
    pub card_cvc: Option<Secret<String>>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UpdatePaymentMethodResponse {
    pub payment_method_id: String,
}

impl TryFrom<&UpdatePaymentMethodV1Request> for ModularPMUpdateRequest {
    type Error = MicroserviceClientError;

    fn try_from(value: &UpdatePaymentMethodV1Request) -> Result<Self, Self::Error> {
        Ok(Self {
            // payment_method_id: value.payment_method_id.clone(),
            payment_method_data: value.payment_method_data.clone(),
            connector_token_details: value.connector_token_details.clone(),
            network_transaction_id: value.network_transaction_id.clone(),
        })
    }
}

impl TryFrom<ModularPaymentMethodResponse> for UpdatePaymentMethodResponse {
    type Error = MicroserviceClientError;

    fn try_from(resp: ModularPaymentMethodResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            payment_method_id: resp.id,
        })
    }
}

impl UpdatePaymentMethod {
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
    v2_response = ModularPaymentMethodResponse,
    v1_response = UpdatePaymentMethodResponse,
    client = crate::client::PaymentMethodClient<'_>,
    body = UpdatePaymentMethod::build_body,
    path_params = UpdatePaymentMethod::build_path_params
);
