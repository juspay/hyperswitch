use common_enums::AttemptStatus;
use common_utils::errors::CustomResult;
use common_utils::id_type;
use hyperswitch_domain_models::{
    router_data::ErrorResponse, router_response_types::PaymentsResponseData,
};
use unified_connector_service_client::payments::{
    self as payments_grpc, PaymentServiceAuthorizeRequest, PaymentServiceAuthorizeResponse,
    PaymentServiceGetRequest, PaymentServiceGetResponse, PaymentServiceRegisterRequest,
    PaymentServiceRegisterResponse, PaymentServiceRepeatEverythingRequest,
    PaymentServiceRepeatEverythingResponse, PaymentServiceTransformRequest,
    PaymentServiceTransformResponse,
};

use crate::helpers::ForeignTryFrom;
use async_trait::async_trait;
/// Unified Connector Service (UCS) related transformers
pub mod transformers;

pub use transformers::WebhookTransformData;

/// Type alias for return type used by unified connector service response handlers
type UnifiedConnectorServiceResult = CustomResult<
    (
        Result<(PaymentsResponseData, AttemptStatus), ErrorResponse>,
        u16,
    ),
    transformers::UnifiedConnectorServiceError,
>;

/// Connector authentication metadata required for UCS calls
#[derive(Debug, Clone)]
#[allow(missing_docs)]

pub struct UcsConnectorAuthMetadata {
    pub connector_name: String,
    pub auth_type: String,
    pub api_key: Option<masking::Secret<String>>,
    pub key1: Option<masking::Secret<String>>,
    pub api_secret: Option<masking::Secret<String>>,
    pub auth_key_map: Option<
        std::collections::HashMap<
            common_enums::enums::Currency,
            common_utils::pii::SecretSerdeValue,
        >,
    >,
    pub merchant_id: masking::Secret<String>,
}
#[derive(Debug, serde::Serialize, Clone)]
#[allow(missing_docs)]
pub struct LineageIds {
    pub merchant_id: id_type::MerchantId,
    pub profile_id: id_type::ProfileId,
}
/// Headers required for UCS gRPC calls
#[derive(Debug, Clone)]
#[allow(missing_docs)]
pub struct UcsHeaders {
    pub tenant_id: String,
    pub request_id: Option<String>,
    pub lineage_ids: LineageIds, // URL-encoded lineage ids
    pub external_vault_proxy_metadata: Option<String>,
    pub merchant_reference_id: Option<String>,
    pub shadow_mode: Option<bool>,
}

#[allow(missing_docs)]
pub fn handle_unified_connector_service_response_for_payment_get(
    response: PaymentServiceGetResponse,
) -> UnifiedConnectorServiceResult {
    let status_code = transformers::convert_connector_service_status_code(response.status_code)?;

    let router_data_response =
        Result::<(PaymentsResponseData, AttemptStatus), ErrorResponse>::foreign_try_from(response)?;

    Ok((router_data_response, status_code))
}

#[async_trait]
#[allow(missing_docs)]
pub trait UnifiedConnectorServiceInterface: Send + Sync {
    /// Performs Payment Authorization
    async fn payment_authorize(
        &self,
        request: PaymentServiceAuthorizeRequest,
        connector_auth_metadata: UcsConnectorAuthMetadata,
        headers: UcsHeaders,
    ) -> CustomResult<PaymentServiceAuthorizeResponse, transformers::UnifiedConnectorServiceError>;

    /// Performs Payment Get/Sync
    async fn payment_get(
        &self,
        request: PaymentServiceGetRequest,
        connector_auth_metadata: UcsConnectorAuthMetadata,
        headers: UcsHeaders,
    ) -> CustomResult<PaymentServiceGetResponse, transformers::UnifiedConnectorServiceError>;

    /// Performs Payment Setup Mandate
    async fn payment_setup_mandate(
        &self,
        request: PaymentServiceRegisterRequest,
        connector_auth_metadata: UcsConnectorAuthMetadata,
        headers: UcsHeaders,
    ) -> CustomResult<PaymentServiceRegisterResponse, transformers::UnifiedConnectorServiceError>;

    /// Performs Payment Repeat (MIT - Merchant Initiated Transaction)
    async fn payment_repeat(
        &self,
        request: PaymentServiceRepeatEverythingRequest,
        connector_auth_metadata: UcsConnectorAuthMetadata,
        headers: UcsHeaders,
    ) -> CustomResult<
        PaymentServiceRepeatEverythingResponse,
        transformers::UnifiedConnectorServiceError,
    >;

    /// Transforms incoming webhook through UCS
    async fn transform_incoming_webhook(
        &self,
        request: PaymentServiceTransformRequest,
        connector_auth_metadata: UcsConnectorAuthMetadata,
        headers: UcsHeaders,
    ) -> CustomResult<PaymentServiceTransformResponse, transformers::UnifiedConnectorServiceError>;
}
