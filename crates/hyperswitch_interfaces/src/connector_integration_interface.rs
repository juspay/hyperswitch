use api_models::webhooks::{IncomingWebhookEvent, ObjectReferenceId};
use common_enums::PaymentAction;
use common_utils::{crypto, errors::CustomResult, request::Request};
use hyperswitch_domain_models::{
    api::ApplicationResponse,
    connector_endpoints::Connectors,
    errors::api_error_response::ApiErrorResponse,
    payment_method_data::PaymentMethodData,
    router_data::{ConnectorAuthType, ErrorResponse, RouterData},
    router_data_v2::RouterDataV2,
    router_response_types::{ConnectorInfo, SupportedPaymentMethods},
};

use crate::{
    api,
    api::{
        BoxedConnectorIntegration, CaptureSyncMethod, Connector, ConnectorAccessTokenSuffix,
        ConnectorCommon, ConnectorIntegration, ConnectorRedirectResponse, ConnectorSpecifications,
        ConnectorValidation, CurrencyUnit,
    },
    authentication::ExternalAuthenticationPayload,
    connector_integration_v2::{BoxedConnectorIntegrationV2, ConnectorIntegrationV2, ConnectorV2},
    disputes, errors,
    events::connector_api_logs::ConnectorEvent,
    types,
    webhooks::{IncomingWebhook, IncomingWebhookFlowError, IncomingWebhookRequestDetails},
};

/// RouterDataConversion trait
///
/// This trait must be implemented for conversion between Router data and RouterDataV2
pub trait RouterDataConversion<T, Req: Clone, Resp: Clone> {
    /// Convert RouterData to RouterDataV2
    ///
    /// # Arguments
    ///
    /// * `old_router_data` - A reference to the old RouterData
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the new RouterDataV2 or a ConnectorError
    fn from_old_router_data(
        old_router_data: &RouterData<T, Req, Resp>,
    ) -> CustomResult<RouterDataV2<T, Self, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized;
    /// Convert RouterDataV2 back to RouterData
    ///
    /// # Arguments
    ///
    /// * `new_router_data` - The new RouterDataV2
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the old RouterData or a ConnectorError
    fn to_old_router_data(
        new_router_data: RouterDataV2<T, Self, Req, Resp>,
    ) -> CustomResult<RouterData<T, Req, Resp>, errors::ConnectorError>
    where
        Self: Sized;
}
/// Alias for Box<&'static (dyn Connector + Sync)>
pub type BoxedConnector = Box<&'static (dyn Connector + Sync)>;
/// Alias for Box<&'static (dyn ConnectorV2 + Sync)>
pub type BoxedConnectorV2 = Box<&'static (dyn ConnectorV2 + Sync)>;

/// Enum representing the Connector
#[derive(Clone)]
pub enum ConnectorEnum {
    /// Old connector type
    Old(BoxedConnector),
    /// New connector type
    New(BoxedConnectorV2),
}

impl std::fmt::Debug for ConnectorEnum {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Old(_) => f
                .debug_tuple("Old")
                .field(&std::any::type_name::<BoxedConnector>().to_string())
                .finish(),
            Self::New(_) => f
                .debug_tuple("New")
                .field(&std::any::type_name::<BoxedConnectorV2>().to_string())
                .finish(),
        }
    }
}

#[allow(missing_debug_implementations)]
/// Enum representing the Connector Integration
#[derive(Clone)]
pub enum ConnectorIntegrationEnum<'a, F, ResourceCommonData, Req, Resp> {
    /// Old connector integration type
    Old(BoxedConnectorIntegration<'a, F, Req, Resp>),
    /// New connector integration type
    New(BoxedConnectorIntegrationV2<'a, F, ResourceCommonData, Req, Resp>),
}

/// Alias for Box<dyn ConnectorIntegrationInterface>
pub type BoxedConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp> =
    Box<dyn ConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp> + Send + Sync>;

impl ConnectorEnum {
    /// Get the connector integration
    ///
    /// # Returns
    ///
    /// A `BoxedConnectorIntegrationInterface` containing the connector integration
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
    /// validates the file upload
    pub fn validate_file_upload(
        &self,
        purpose: api::files::FilePurpose,
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
impl IncomingWebhook for ConnectorEnum {
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

    #[cfg(feature = "payouts")]
    fn get_payout_webhook_details(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::PayoutWebhookUpdate, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.get_payout_webhook_details(request),
            Self::New(connector) => connector.get_payout_webhook_details(request),
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
    ) -> CustomResult<ApplicationResponse<serde_json::Value>, errors::ConnectorError> {
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

    fn get_additional_payment_method_data(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        Option<api_models::payment_methods::PaymentMethodUpdate>,
        errors::ConnectorError,
    > {
        match self {
            Self::Old(connector) => connector.get_additional_payment_method_data(request),
            Self::New(connector) => connector.get_additional_payment_method_data(request),
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
    fn get_subscription_mit_payment_data(
        &self,
        request: &IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<
        hyperswitch_domain_models::router_flow_types::SubscriptionMitPaymentData,
        errors::ConnectorError,
    > {
        match self {
            Self::Old(connector) => connector.get_subscription_mit_payment_data(request),
            Self::New(connector) => connector.get_subscription_mit_payment_data(request),
        }
    }
}

impl ConnectorRedirectResponse for ConnectorEnum {
    fn get_flow_type(
        &self,
        query_params: &str,
        json_payload: Option<serde_json::Value>,
        action: PaymentAction,
    ) -> CustomResult<common_enums::CallConnectorAction, errors::ConnectorError> {
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
        pm_data: PaymentMethodData,
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
    fn decide_should_continue_after_preprocessing(
        &self,
        current_flow: api::CurrentFlowInfo<'_>,
        pre_processing_flow_name: api::PreProcessingFlowName,
        preprocessing_flow_response: api::PreProcessingFlowResponse<'_>,
    ) -> bool {
        match self {
            Self::Old(connector) => connector.decide_should_continue_after_preprocessing(
                current_flow,
                pre_processing_flow_name,
                preprocessing_flow_response,
            ),
            Self::New(connector) => connector.decide_should_continue_after_preprocessing(
                current_flow,
                pre_processing_flow_name,
                preprocessing_flow_response,
            ),
        }
    }
    fn get_preprocessing_flow_if_needed(
        &self,
        current_flow_info: api::CurrentFlowInfo<'_>,
    ) -> Option<api::PreProcessingFlowName> {
        match self {
            Self::Old(connector) => connector.get_preprocessing_flow_if_needed(current_flow_info),
            Self::New(connector) => connector.get_preprocessing_flow_if_needed(current_flow_info),
        }
    }
    fn get_alternate_flow_if_needed(
        &self,
        current_flow: api::CurrentFlowInfo<'_>,
    ) -> Option<api::AlternateFlow> {
        match self {
            Self::Old(connector) => connector.get_alternate_flow_if_needed(current_flow),
            Self::New(connector) => connector.get_alternate_flow_if_needed(current_flow),
        }
    }

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

    /// Check if connector supports authentication token
    fn authentication_token_for_token_creation(&self) -> bool {
        match self {
            Self::Old(connector) => connector.authentication_token_for_token_creation(),
            Self::New(connector) => connector.authentication_token_for_token_creation(),
        }
    }

    /// If connector supports session token generation
    fn is_sdk_client_token_generation_enabled(&self) -> bool {
        match self {
            Self::Old(connector) => connector.is_sdk_client_token_generation_enabled(),
            Self::New(connector) => connector.is_sdk_client_token_generation_enabled(),
        }
    }

    /// Supported payment methods for session token generation
    fn supported_payment_method_types_for_sdk_client_token_generation(
        &self,
    ) -> Vec<common_enums::PaymentMethodType> {
        match self {
            Self::Old(connector) => {
                connector.supported_payment_method_types_for_sdk_client_token_generation()
            }
            Self::New(connector) => {
                connector.supported_payment_method_types_for_sdk_client_token_generation()
            }
        }
    }

    /// Validate whether session token is generated for payment payment type
    fn validate_sdk_session_token_for_payment_method(
        &self,
        current_core_payment_method_type: &common_enums::PaymentMethodType,
    ) -> bool {
        match self {
            Self::Old(connector) => connector
                .validate_sdk_session_token_for_payment_method(current_core_payment_method_type),
            Self::New(connector) => connector
                .validate_sdk_session_token_for_payment_method(current_core_payment_method_type),
        }
    }

    #[cfg(feature = "v1")]
    fn generate_connector_request_reference_id(
        &self,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
        is_config_enabled_to_send_payment_id_as_connector_request_id: bool,
    ) -> String {
        match self {
            Self::Old(connector) => connector.generate_connector_request_reference_id(
                payment_intent,
                payment_attempt,
                is_config_enabled_to_send_payment_id_as_connector_request_id,
            ),
            Self::New(connector) => connector.generate_connector_request_reference_id(
                payment_intent,
                payment_attempt,
                is_config_enabled_to_send_payment_id_as_connector_request_id,
            ),
        }
    }

    #[cfg(feature = "v2")]
    /// Generate connector request reference ID
    fn generate_connector_request_reference_id(
        &self,
        payment_intent: &hyperswitch_domain_models::payments::PaymentIntent,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> String {
        match self {
            Self::Old(connector) => {
                connector.generate_connector_request_reference_id(payment_intent, payment_attempt)
            }
            Self::New(connector) => {
                connector.generate_connector_request_reference_id(payment_intent, payment_attempt)
            }
        }
    }

    #[cfg(feature = "v1")]
    fn generate_connector_customer_id(
        &self,
        customer_id: &Option<common_utils::id_type::CustomerId>,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> Option<String> {
        match self {
            Self::Old(connector) => {
                connector.generate_connector_customer_id(customer_id, merchant_id)
            }
            Self::New(connector) => {
                connector.generate_connector_customer_id(customer_id, merchant_id)
            }
        }
    }

    #[cfg(feature = "v2")]
    fn generate_connector_customer_id(
        &self,
        customer_id: &Option<common_utils::id_type::CustomerId>,
        merchant_id: &common_utils::id_type::MerchantId,
    ) -> Option<String> {
        todo!()
    }

    /// Check if connector requires create customer call
    fn should_call_connector_customer(
        &self,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> bool {
        match self {
            Self::Old(connector) => connector.should_call_connector_customer(payment_attempt),
            Self::New(connector) => connector.should_call_connector_customer(payment_attempt),
        }
    }

    fn should_call_tokenization_before_setup_mandate(&self) -> bool {
        match self {
            Self::Old(connector) => connector.should_call_tokenization_before_setup_mandate(),
            Self::New(connector) => connector.should_call_tokenization_before_setup_mandate(),
        }
    }
}

impl ConnectorCommon for ConnectorEnum {
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
        auth_type: &ConnectorAuthType,
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        match self {
            Self::Old(connector) => connector.build_error_response(res, event_builder),
            Self::New(connector) => connector.build_error_response(res, event_builder),
        }
    }
}

impl ConnectorAccessTokenSuffix for ConnectorEnum {
    fn get_access_token_key<F, Req, Res>(
        &self,
        router_data: &RouterData<F, Req, Res>,
        merchant_connector_id_or_connector_name: String,
    ) -> CustomResult<String, errors::ConnectorError> {
        match self {
            Self::Old(connector) => {
                connector.get_access_token_key(router_data, merchant_connector_id_or_connector_name)
            }
            Self::New(connector) => {
                connector.get_access_token_key(router_data, merchant_connector_id_or_connector_name)
            }
        }
    }
}

/// Trait representing the connector integration interface
///
/// This trait defines the methods required for a connector integration interface.
pub trait ConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp>: Send + Sync {
    /// Clone the connector integration interface
    ///
    /// # Returns
    ///
    /// A `Box` containing the cloned connector integration interface
    fn clone_box(
        &self,
    ) -> Box<dyn ConnectorIntegrationInterface<F, ResourceCommonData, Req, Resp> + Send + Sync>;
    /// Get the multiple capture sync method
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the `CaptureSyncMethod` or a `ConnectorError`
    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<CaptureSyncMethod, errors::ConnectorError>;
    /// Build a request for the connector integration
    ///
    /// # Arguments
    ///
    /// * `req` - A reference to the RouterData
    /// # Returns
    ///
    /// A `CustomResult` containing an optional Request or a ConnectorError
    fn build_request(
        &self,
        req: &RouterData<F, Req, Resp>,
        _connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError>;
    /// handles response from the connector
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
    /// Get the error response
    ///
    /// # Arguments
    ///
    /// * `res` - The response
    /// * `event_builder` - An optional event builder
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the `ErrorResponse` or a `ConnectorError`
    fn get_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError>;
    /// Get the 5xx error response
    ///
    /// # Arguments
    ///
    /// * `res` - The response
    /// * `event_builder` - An optional event builder
    ///
    /// # Returns
    ///
    /// A `CustomResult` containing the `ErrorResponse` or a `ConnectorError`
    fn get_5xx_error_response(
        &self,
        res: types::Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError>;
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
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
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
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

impl api::ConnectorTransactionId for ConnectorEnum {
    /// Get the connector transaction ID
    ///
    /// # Arguments
    ///
    /// * `payment_attempt` - The payment attempt
    ///
    /// # Returns
    ///
    /// A `Result` containing an optional transaction ID or an ApiErrorResponse
    fn connector_transaction_id(
        &self,
        payment_attempt: &hyperswitch_domain_models::payments::payment_attempt::PaymentAttempt,
    ) -> Result<Option<String>, ApiErrorResponse> {
        match self {
            Self::Old(connector) => connector.connector_transaction_id(payment_attempt),
            Self::New(connector) => connector.connector_transaction_id(payment_attempt),
        }
    }
}
