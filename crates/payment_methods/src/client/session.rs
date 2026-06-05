//! Payment method session create flow types and models.

use common_utils::{
    id_type,
    request::{Method, RequestContent},
};
use hyperswitch_interfaces::micro_service::MicroserviceClientError;
use hyperswitch_masking::Secret;
use serde::{Deserialize, Serialize};

/// Flow type for creating a payment method session via the internal PM service.
#[derive(Debug)]
pub struct CreatePaymentMethodSession;

/// Request sent to the internal PM service to create a payment method session.
#[derive(Debug)]
pub struct CreatePaymentMethodSessionV1Request {
    /// Customer id for which the session is created (v2 global customer id as string).
    pub customer_id: Option<id_type::CustomerId>,
    /// Prefix that forms part of the URL path (e.g. "v2").
    pub modular_service_prefix: String,
    /// Storage type for the session.
    pub storage_type: common_enums::StorageType,
}

/// Wire-level request body serialised as JSON.
#[derive(Debug, Clone, Serialize)]
pub struct ModularPMSessionCreateRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<id_type::CustomerId>,
    pub storage_type: common_enums::StorageType,
}

/// Local deserializable mirror of `api_models::payments::VgsSessionDetails`.
#[derive(Debug, Clone, Deserialize)]
pub struct VgsSessionDetailsResponse {
    pub external_vault_id: Secret<String>,
    pub sdk_env: String,
}

/// Local deserializable mirror of `api_models::payments::HyperswitchVaultSessionDetails`.
#[derive(Debug, Clone, Deserialize)]
pub struct HyperswitchVaultSessionDetailsResponse {
    pub payment_method_session_id: Secret<String>,
    pub client_secret: Secret<String>,
    pub publishable_key: Secret<String>,
    pub profile_id: Secret<String>,
}

/// Local deserializable mirror of `api_models::payments::VaultSessionDetails`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultSessionDetailsResponse {
    Vgs(VgsSessionDetailsResponse),
    HyperswitchVault(HyperswitchVaultSessionDetailsResponse),
}

impl From<VaultSessionDetailsResponse> for api_models::payments::VaultSessionDetails {
    fn from(resp: VaultSessionDetailsResponse) -> Self {
        match resp {
            VaultSessionDetailsResponse::Vgs(vgs) => {
                Self::Vgs(api_models::payments::VgsSessionDetails {
                    external_vault_id: vgs.external_vault_id,
                    sdk_env: vgs.sdk_env,
                })
            }
            VaultSessionDetailsResponse::HyperswitchVault(hsv) => {
                Self::HyperswitchVault(api_models::payments::HyperswitchVaultSessionDetails {
                    payment_method_session_id: hsv.payment_method_session_id,
                    client_secret: hsv.client_secret,
                    publishable_key: hsv.publishable_key,
                    profile_id: hsv.profile_id,
                })
            }
        }
    }
}

/// Minimal subset of the PM session response we care about.
#[derive(Debug, Clone, Deserialize)]
pub struct ModularPMSessionCreateResponse {
    /// The payment method session ID.
    pub id: String,
    /// The client secret for this session.
    pub client_secret: Option<Secret<String>>,
    /// The customer ID associated with this session.
    pub customer_id: Option<id_type::CustomerId>,
    /// The pre-computed SDK authorization string (base64-encoded).
    pub sdk_authorization: Option<String>,
    /// External vault session details returned by the PM service when an external vault is
    /// configured for the profile.
    pub external_vault_details: Option<VaultSessionDetailsResponse>,
}

/// V1-facing response (thin wrapper around the wire response).
#[derive(Debug, Clone)]
pub struct CreatePaymentMethodSessionResponse {
    pub id: String,
    pub client_secret: Option<Secret<String>>,
    pub customer_id: Option<id_type::CustomerId>,
    pub sdk_authorization: Option<String>,
    pub external_vault_details: Option<api_models::payments::VaultSessionDetails>,
}

// --- Conversions ---

impl TryFrom<&CreatePaymentMethodSessionV1Request> for ModularPMSessionCreateRequest {
    type Error = MicroserviceClientError;

    fn try_from(req: &CreatePaymentMethodSessionV1Request) -> Result<Self, Self::Error> {
        Ok(Self {
            customer_id: req.customer_id.clone(),
            storage_type: req.storage_type,
        })
    }
}

impl TryFrom<ModularPMSessionCreateResponse> for CreatePaymentMethodSessionResponse {
    type Error = MicroserviceClientError;

    fn try_from(resp: ModularPMSessionCreateResponse) -> Result<Self, Self::Error> {
        Ok(Self {
            id: resp.id,
            client_secret: resp.client_secret,
            customer_id: resp.customer_id,
            sdk_authorization: resp.sdk_authorization,
            external_vault_details: resp.external_vault_details.map(Into::into),
        })
    }
}

// --- Micro-service flow wiring ---

impl CreatePaymentMethodSession {
    fn build_body(&self, request: ModularPMSessionCreateRequest) -> Option<RequestContent> {
        Some(RequestContent::Json(Box::new(request)))
    }

    fn build_path_params(
        &self,
        request: &CreatePaymentMethodSessionV1Request,
    ) -> Vec<(&'static str, String)> {
        vec![("prefix", request.modular_service_prefix.clone())]
    }
}

hyperswitch_interfaces::impl_microservice_flow!(
    CreatePaymentMethodSession,
    method = Method::Post,
    path = "/{prefix}/payment-method-sessions",
    v1_request = CreatePaymentMethodSessionV1Request,
    v2_request = ModularPMSessionCreateRequest,
    v2_response = ModularPMSessionCreateResponse,
    v1_response = CreatePaymentMethodSessionResponse,
    client = crate::client::PaymentMethodClient<'_>,
    body = CreatePaymentMethodSession::build_body,
    path_params = CreatePaymentMethodSession::build_path_params
);
