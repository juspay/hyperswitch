use common_utils::{crypto, errors::CustomResult, request::Request};
use hyperswitch_domain_models::{
    router_data::RouterData,
    router_data_v2::RouterDataV2,
    router_response_types::{ConnectorInfo, SupportedPaymentMethods},
};
use hyperswitch_interfaces::{
    authentication::ExternalAuthenticationPayload,
    connector_integration_v2::ConnectorIntegrationV2, webhooks::IncomingWebhookFlowError,
};

use super::{BoxedConnectorIntegrationV2, ConnectorSpecifications, ConnectorValidation};
use crate::{
    core::payments,
    errors,
    events::connector_api_logs::ConnectorEvent,
    services::{
        api as services_api, BoxedConnectorIntegration, CaptureSyncMethod, ConnectorIntegration,
        ConnectorRedirectResponse, PaymentAction,
    },
    settings::Connectors,
    types::{
        self,
        api::{
            self, disputes, Connector, ConnectorV2, CurrencyUnit, IncomingWebhookEvent,
            IncomingWebhookRequestDetails, ObjectReferenceId,
        },
        domain,
    },
};
pub trait RouterDataConversion<T, Req: Clone, Resp: Clone> {
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized;
    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized;
}

#[derive(Clone)]
pub enum ConnectorEnum {
    Old(api::BoxedConnector),
    New(api::BoxedConnectorV2),
}

#[derive(Clone)]
pub enum ConnectorIntegrationEnum<'a, F, ResourceCommonData, Req, Resp> {
    Old(BoxedConnectorIntegration<'a, F, Req, Resp>),
    New(BoxedConnectorIntegrationV2<'a, F, ResourceCommonData, Req, Resp>),
}

pub type BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp> =
    Box<dyn ConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp> + Send + Sync>;

impl ConnectorEnum {
    pub fn get_connector_integration<F, ResourceCommonData, Req, Resp>(
        &self,
    ) -> BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>
    where
        dyn Connector + Sync: ConnectorIntegration<F, Req, Resp>,
        dyn ConnectorV2 + Sync: ConnectorIntegrationV2<F, ResourceCommonData, Req, Resp>,
        ResourceCommonData: RouterDataConversion<F, Req, Resp> + Clone + 'static,
        F: Clone + 'static,
        Req: Clone + 'static,
        Resp: Clone + 'static,
    {
        match self {
            Self::Old(old_integration) => Box::new(ConnectorIntegrationEnum::Old(
                old_integration.get_connector_integration(),
            )),
            Self::New(new_integration) => Box::new(ConnectorIntegrationEnum::New(
                new_integration.get_connector_integration_v2(),
            )),
        }
    }

    pub fn validate_file_upload(
        &self,
        purpose: api::FilePurpose,
        file_size: i32,
        file_type: mime::Mime,
    ) -> CustomResult<(), errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.validate_file_upload(purpose, file_size, file_type),
            Self::New(connector) => {
                connector.validate_file_upload_v2(purpose, file_size, file_type)
            }
        }
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for ConnectorEnum {
    fn get_webhook_body_decoding_algorithm(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::DecodeMessage + Send>, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_webhook_body_decoding_algorithm(request),
            Self::New(connector) => connector.get_webhook_body_decoding_algorithm(request),
        }
    }

    fn get_webhook_body_decoding_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_webhook_body_decoding_message(request),
            Self::New(connector) => connector.get_webhook_body_decoding_message(request),
        }
    }

    async fn decode_webhook_body(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        connector_name: &str,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        match self {
            Self::Old(connector) => {
                connector
                    .decode_webhook_body(
                        request,
                        merchant_id,
                        connector_webhook_details,
                        connector_name,
                    )
                    .await
            }
            Self::New(connector) => {
                connector
                    .decode_webhook_body(
                        request,
                        merchant_id,
                        connector_webhook_details,
                        connector_name,
                    )
                    .await
            }
        }
    }

    fn get_webhook_source_verification_algorithm(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_webhook_source_verification_algorithm(request),
            Self::New(connector) => connector.get_webhook_source_verification_algorithm(request),
        }
    }

    async fn get_webhook_source_verification_merchant_secret(
        &self,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_name: &str,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<api_models::webhooks::ConnectorWebhookSecrets, errors::ConnectorError> {
        match self {
            Self::Old(connector) => {
                connector
                    .get_webhook_source_verification_merchant_secret(
                        merchant_id,
                        connector_name,
                        connector_webhook_details,
                    )
                    .await
            }
            Self::New(connector) => {
                connector
                    .get_webhook_source_verification_merchant_secret(
                        merchant_id,
                        connector_name,
                        connector_webhook_details,
                    )
                    .await
            }
        }
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector
                .get_webhook_source_verification_signature(request, connector_webhook_secrets),
            Self::New(connector) => connector
                .get_webhook_source_verification_signature(request, connector_webhook_secrets),
        }
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_webhook_source_verification_message(
                request,
                merchant_id,
                connector_webhook_secrets,
            ),
            Self::New(connector) => connector.get_webhook_source_verification_message(
                request,
                merchant_id,
                connector_webhook_secrets,
            ),
        }
    }

    async fn verify_webhook_source(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        merchant_id: &common_utils::id_type::MerchantId,
        connector_webhook_details: Option<common_utils::pii::SecretSerdeValue>,
        connector_account_details: crypto::Encryptable<masking::Secret<serde_json::Value>>,
        connector_name: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        match self {
            Self::Old(connector) => {
                connector
                    .verify_webhook_source(
                        request,
                        merchant_id,
                        connector_webhook_details,
                        connector_account_details,
                        connector_name,
                    )
                    .await
            }
            Self::New(connector) => {
                connector
                    .verify_webhook_source(
                        request,
                        merchant_id,
                        connector_webhook_details,
                        connector_account_details,
                        connector_name,
                    )
                    .await
            }
        }
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ObjectReferenceId, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_webhook_object_reference_id(request),
            Self::New(connector) => connector.get_webhook_object_reference_id(request),
        }
    }

    fn get_webhook_event_type(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_webhook_event_type(request),
            Self::New(connector) => connector.get_webhook_event_type(request),
        }
    }

    fn get_webhook_resource_object(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_webhook_resource_object(request),
            Self::New(connector) => connector.get_webhook_resource_object(request),
        }
    }

    fn get_webhook_api_response(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
        error_kind: Option<IncomingWebhookFlowError>,
    ) -> CustomResult<services_api::ApplicationResponse<serde_json::Value>, errors::ConnectorError>
    {
        match self {
            Self::Old(connector) => connector.get_webhook_api_response(request, error_kind),
            Self::New(connector) => connector.get_webhook_api_response(request, error_kind),
        }
    }

    fn get_dispute_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<disputes::DisputePayload, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_dispute_details(request),
            Self::New(connector) => connector.get_dispute_details(request),
        }
    }

    fn get_external_authentication_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<ExternalAuthenticationPayload, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_external_authentication_details(request),
            Self::New(connector) => connector.get_external_authentication_details(request),
        }
    }

    fn get_mandate_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        Option<hyperswitch_domain_models::router_flow_types::ConnectorMandateDetails>,
        errors::ConnectorError,
    > {
        match self {
            Self::Old(connector) => connector.get_mandate_details(request),
            Self::New(connector) => connector.get_mandate_details(request),
        }
    }

    fn get_network_txn_id(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        Option<hyperswitch_domain_models::router_flow_types::ConnectorNetworkTxnId>,
        errors::ConnectorError,
    > {
        match self {
            Self::Old(connector) => connector.get_network_txn_id(request),
            Self::New(connector) => connector.get_network_txn_id(request),
        }
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_attempt_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        hyperswitch_domain_models::revenue_recovery::RevenueRecoveryAttemptData,
        errors::ConnectorError,
    > {
        match self {
            Self::Old(connector) => connector.get_revenue_recovery_attempt_details(request),
            Self::New(connector) => connector.get_revenue_recovery_attempt_details(request),
        }
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_revenue_recovery_invoice_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        hyperswitch_domain_models::revenue_recovery::RevenueRecoveryInvoiceData,
        errors::ConnectorError,
    > {
        match self {
            Self::Old(connector) => connector.get_revenue_recovery_invoice_details(request),
            Self::New(connector) => connector.get_revenue_recovery_invoice_details(request),
        }
    }
}

impl api::ConnectorTransactionId for ConnectorEnum {
    fn connector_transaction_id(
        &self,
        payment_attempt: hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> Result<Option<String>, errors::ApiErrorResponse> {
        match self {
            Self::Old(connector) => connector.connector_transaction_id(payment_attempt),
            Self::New(connector) => connector.connector_transaction_id(payment_attempt),
        }
    }
}

impl ConnectorRedirectResponse for ConnectorEnum {
    fn get_flow_type(
        &self,
        query_params: &str,
        json_payload: Option<serde_json::Value>,
        action: PaymentAction,
    ) -> CustomResult<payments::CallConnectorAction, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_flow_type(query_params, json_payload, action),
            Self::New(connector) => connector.get_flow_type(query_params, json_payload, action),
        }
    }
}

impl ConnectorValidation for ConnectorEnum {
    fn validate_connector_against_payment_request(
        &self,
        capture_method: Option<common_enums::CaptureMethod>,
        payment_method: common_enums::PaymentMethod,
        pmt: Option<common_enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.validate_connector_against_payment_request(
                capture_method,
                payment_method,
                pmt,
            ),
            Self::New(connector) => connector.validate_connector_against_payment_request(
                capture_method,
                payment_method,
                pmt,
            ),
        }
    }

    fn validate_mandate_payment(
        &self,
        pm_type: Option<common_enums::PaymentMethodType>,
        pm_data: domain::payments::PaymentMethodData,
    ) -> CustomResult<(), errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.validate_mandate_payment(pm_type, pm_data),
            Self::New(connector) => connector.validate_mandate_payment(pm_type, pm_data),
        }
    }

    fn validate_psync_reference_id(
        &self,
        data: &hyperswitch_domain_models::router_request_types::PaymentsSyncData,
        is_three_ds: bool,
        status: common_enums::enums::AttemptStatus,
        connector_meta_data: Option<common_utils::pii::SecretSerdeValue>,
    ) -> CustomResult<(), errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.validate_psync_reference_id(
                data,
                is_three_ds,
                status,
                connector_meta_data,
            ),
            Self::New(connector) => connector.validate_psync_reference_id(
                data,
                is_three_ds,
                status,
                connector_meta_data,
            ),
        }
    }

    fn is_webhook_source_verification_mandatory(&self) -> bool {
        match self {
            Self::Old(connector) => connector.is_webhook_source_verification_mandatory(),
            Self::New(connector) => connector.is_webhook_source_verification_mandatory(),
        }
    }
}

impl ConnectorSpecifications for ConnectorEnum {
    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        match self {
            Self::Old(connector) => connector.get_supported_payment_methods(),
            Self::New(connector) => connector.get_supported_payment_methods(),
        }
    }

    /// Supported webhooks flows
    fn get_supported_webhook_flows(&self) -> Option<&'static [common_enums::EventClass]> {
        match self {
            Self::Old(connector) => connector.get_supported_webhook_flows(),
            Self::New(connector) => connector.get_supported_webhook_flows(),
        }
    }

    /// Details related to connector
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        match self {
            Self::Old(connector) => connector.get_connector_about(),
            Self::New(connector) => connector.get_connector_about(),
        }
    }
}

impl api::ConnectorCommon for ConnectorEnum {
    fn id(&self) -> &'static str {
        match self {
            Self::Old(connector) => connector.id(),
            Self::New(connector) => connector.id(),
        }
    }

    fn get_currency_unit(&self) -> CurrencyUnit {
        match self {
            Self::Old(connector) => connector.get_currency_unit(),
            Self::New(connector) => connector.get_currency_unit(),
        }
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_auth_header(auth_type),
            Self::New(connector) => connector.get_auth_header(auth_type),
        }
    }

    fn common_get_content_type(&self) -> &'static str {
        match self {
            Self::Old(connector) => connector.common_get_content_type(),
            Self::New(connector) => connector.common_get_content_type(),
        }
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        match self {
            Self::Old(connector) => connector.base_url(connectors),
            Self::New(connector) => connector.base_url(connectors),
        }
    }

    fn build_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.build_error_response(res, event_builder),
            Self::New(connector) => connector.build_error_response(res, event_builder),
        }
    }
}

pub trait ConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>: Send + Sync {
    fn clone_box(
        &self,
    ) -> Box<dyn ConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp> + Send + Sync>;
    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<CaptureSyncMethod, errors::ConnectorError>;
    fn build_request(
        &self,
        req: &RouterData<F, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError>;
    fn handle_response(
        &self,
        data: &RouterData<F, Req, Resp>,
        event_builder: Option<&mut ConnectorEvent>,
        _res: types::Response,
    ) -> CustomResult<RouterData<F, Req, Resp>, errors::ConnectorError>
    where
        F: Clone,
        Req: Clone,
        Resp: Clone;
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError>;
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError>;
}

impl<T: 'static, ResourceCommonData: 'static, Req: 'static, Resp: 'static>
    ConnectorIntegrationInterface<T, ResourceCommonData, Req, Resp>
    for ConnectorIntegrationEnum<'static, T, ResourceCommonData, Req, Resp>
where
    ResourceCommonData: RouterDataConversion<T, Req, Resp> + Clone,
    T: Clone,
    Req: Clone,
    Resp: Clone,
{
    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<CaptureSyncMethod, errors::ConnectorError> {
        match self {
            ConnectorIntegrationEnum::Old(old_integration) => {
                old_integration.get_multiple_capture_sync_method()
            }
            ConnectorIntegrationEnum::New(new_integration) => {
                new_integration.get_multiple_capture_sync_method()
            }
        }
    }
    fn build_request(
        &self,
        req: &RouterData<T, Req, Resp>,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        match self {
            ConnectorIntegrationEnum::Old(old_integration) => {
                old_integration.build_request(req, connectors)
            }
            ConnectorIntegrationEnum::New(new_integration) => {
                let new_router_data = ResourceCommonData::from_old_router_data(req)?;
                new_integration.build_request_v2(&new_router_data)
            }
        }
    }
    fn handle_response(
        &self,
        data: &RouterData<T, Req, Resp>,
        event_builder: Option<&mut ConnectorEvent>,
        res: types::Response,
    ) -> CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        T: Clone,
        Req: Clone,
        Resp: Clone,
    {
        match self {
            ConnectorIntegrationEnum::Old(old_integration) => {
                old_integration.handle_response(data, event_builder, res)
            }
            ConnectorIntegrationEnum::New(new_integration) => {
                let new_router_data = ResourceCommonData::from_old_router_data(data)?;
                new_integration
                    .handle_response_v2(&new_router_data, event_builder, res)
                    .map(ResourceCommonData::to_old_router_data)?
            }
        }
    }
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        match self {
            ConnectorIntegrationEnum::Old(old_integration) => {
                old_integration.get_error_response(res, event_builder)
            }
            ConnectorIntegrationEnum::New(new_integration) => {
                new_integration.get_error_response_v2(res, event_builder)
            }
        }
    }
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        match self {
            ConnectorIntegrationEnum::Old(old_integration) => {
                old_integration.get_5xx_error_response(res, event_builder)
            }
            ConnectorIntegrationEnum::New(new_integration) => {
                new_integration.get_5xx_error_response(res, event_builder)
            }
        }
    }

    fn clone_box(
        &self,
    ) -> Box<dyn ConnectorIntegrationInterface<T, ResourceCommonData, Req, Resp> + Send + Sync>
    {
        Box::new(self.clone())
    }
}
