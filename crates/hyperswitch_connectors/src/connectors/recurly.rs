pub mod transformers;

use base64::Engine;
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use common_utils::request::{Method, Request, RequestBuilder};
use common_utils::{consts, errors::CustomResult, ext_traits::BytesExt};
#[cfg(feature = "v1")]
use error_stack::report;
use error_stack::ResultExt;
use hyperswitch_domain_models::{
    router_data::{ConnectorAuthType, ErrorResponse},
    router_data_v2::{
        flow_common_types::{
            GetSubscriptionEstimateData, GetSubscriptionPlanPricesData, GetSubscriptionPlansData,
            InvoiceRecordBackData, SubscriptionCancelData, SubscriptionCreateData,
            SubscriptionCustomerData, SubscriptionPauseData, SubscriptionResumeData,
        },
        UasFlowData,
    },
    router_flow_types::{
        subscriptions::{
            GetSubscriptionEstimate, GetSubscriptionPlanPrices, GetSubscriptionPlans,
            SubscriptionCancel, SubscriptionCreate, SubscriptionPause, SubscriptionResume,
        },
        unified_authentication_service::{
            Authenticate, AuthenticationConfirmation, PostAuthenticate, PreAuthenticate,
        },
        CreateConnectorCustomer, InvoiceRecordBack,
    },
    router_request_types::{
        revenue_recovery::InvoiceRecordBackRequest,
        subscriptions::{
            GetSubscriptionEstimateRequest, GetSubscriptionPlanPricesRequest,
            GetSubscriptionPlansRequest, SubscriptionCancelRequest, SubscriptionCreateRequest,
            SubscriptionPauseRequest, SubscriptionResumeRequest,
        },
        unified_authentication_service::{
            UasAuthenticationRequestData, UasAuthenticationResponseData,
            UasConfirmationRequestData, UasPostAuthenticationRequestData,
            UasPreAuthenticationRequestData,
        },
        ConnectorCustomerData,
    },
    router_response_types::{
        revenue_recovery::InvoiceRecordBackResponse,
        subscriptions::{
            GetSubscriptionEstimateResponse, GetSubscriptionPlanPricesResponse,
            GetSubscriptionPlansResponse, SubscriptionCancelResponse, SubscriptionCreateResponse,
            SubscriptionPauseResponse, SubscriptionResumeResponse,
        },
        PaymentsResponseData,
    },
};
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_domain_models::{
    router_data_v2::flow_common_types as recovery_flow_common_types,
    router_flow_types::revenue_recovery as recovery_router_flows,
    router_request_types::revenue_recovery as recovery_request_types,
    router_response_types::revenue_recovery as recovery_response_types,
    types as recovery_router_data_types,
};
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use hyperswitch_interfaces::types;
use hyperswitch_interfaces::{
    api::{self, ConnectorCommon, ConnectorSpecifications, ConnectorValidation},
    configs::Connectors,
    connector_integration_v2::ConnectorIntegrationV2,
    errors,
    events::connector_api_logs::ConnectorEvent,
    types::Response,
    webhooks,
};
use masking::{Mask, PeekInterface};
use transformers as recurly;

use crate::{connectors::recurly::transformers::RecurlyWebhookBody, constants::headers};
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
use crate::{
    connectors::recurly::transformers::{RecurlyRecordStatus, RecurlyRecoveryDetailsData},
    types::ResponseRouterDataV2,
};
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
const STATUS_SUCCESSFUL_ENDPOINT: &str = "mark_successful";
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
const STATUS_FAILED_ENDPOINT: &str = "mark_failed";

const RECURLY_API_VERSION: &str = "application/vnd.recurly.v2021-02-25";

// We don't need an amount converter because we are not using it anywhere in code, but it's important to note that Float Major Unit is the standard format used by Recurly.
#[derive(Clone)]
pub struct Recurly {
    // amount_converter: &'static (dyn AmountConvertor<Output = FloatMajorUnit> + Sync),
}

impl Recurly {
    pub fn new() -> &'static Self {
        &Self {}
    }

    fn get_signature_elements_from_header(
        headers: &actix_web::http::header::HeaderMap,
    ) -> CustomResult<Vec<Vec<u8>>, errors::ConnectorError> {
        let security_header = headers
            .get("recurly-signature")
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        let security_header_str = security_header
            .to_str()
            .change_context(errors::ConnectorError::WebhookSignatureNotFound)?;
        let header_parts: Vec<Vec<u8>> = security_header_str
            .split(',')
            .map(|part| part.trim().as_bytes().to_vec())
            .collect();

        Ok(header_parts)
    }
}

impl api::PayoutsV2 for Recurly {}
impl api::UnifiedAuthenticationServiceV2 for Recurly {}
impl api::UasPreAuthenticationV2 for Recurly {}
impl api::UasPostAuthenticationV2 for Recurly {}
impl api::UasAuthenticationV2 for Recurly {}
impl api::UasAuthenticationConfirmationV2 for Recurly {}
impl
    ConnectorIntegrationV2<
        PreAuthenticate,
        UasFlowData,
        UasPreAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for Recurly
{
    //TODO: implement sessions flow
}

impl
    ConnectorIntegrationV2<
        PostAuthenticate,
        UasFlowData,
        UasPostAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for Recurly
{
    //TODO: implement sessions flow
}
impl
    ConnectorIntegrationV2<
        AuthenticationConfirmation,
        UasFlowData,
        UasConfirmationRequestData,
        UasAuthenticationResponseData,
    > for Recurly
{
    //TODO: implement sessions flow
}
impl
    ConnectorIntegrationV2<
        Authenticate,
        UasFlowData,
        UasAuthenticationRequestData,
        UasAuthenticationResponseData,
    > for Recurly
{
    //TODO: implement sessions flow
}

impl api::revenue_recovery_v2::RevenueRecoveryV2 for Recurly {}
impl api::subscriptions_v2::SubscriptionsV2 for Recurly {}
impl api::subscriptions_v2::GetSubscriptionPlansV2 for Recurly {}
impl api::subscriptions_v2::SubscriptionRecordBackV2 for Recurly {}
impl api::subscriptions_v2::SubscriptionConnectorCustomerV2 for Recurly {}

impl
    ConnectorIntegrationV2<
        GetSubscriptionPlans,
        GetSubscriptionPlansData,
        GetSubscriptionPlansRequest,
        GetSubscriptionPlansResponse,
    > for Recurly
{
}

#[cfg(feature = "v1")]
impl
    ConnectorIntegrationV2<
        InvoiceRecordBack,
        InvoiceRecordBackData,
        InvoiceRecordBackRequest,
        InvoiceRecordBackResponse,
    > for Recurly
{
}
impl
    ConnectorIntegrationV2<
        CreateConnectorCustomer,
        SubscriptionCustomerData,
        ConnectorCustomerData,
        PaymentsResponseData,
    > for Recurly
{
}

impl api::subscriptions_v2::GetSubscriptionPlanPricesV2 for Recurly {}

impl
    ConnectorIntegrationV2<
        GetSubscriptionPlanPrices,
        GetSubscriptionPlanPricesData,
        GetSubscriptionPlanPricesRequest,
        GetSubscriptionPlanPricesResponse,
    > for Recurly
{
}
impl api::subscriptions_v2::SubscriptionsCreateV2 for Recurly {}
impl
    ConnectorIntegrationV2<
        SubscriptionCreate,
        SubscriptionCreateData,
        SubscriptionCreateRequest,
        SubscriptionCreateResponse,
    > for Recurly
{
}

impl api::subscriptions_v2::GetSubscriptionEstimateV2 for Recurly {}
impl
    ConnectorIntegrationV2<
        GetSubscriptionEstimate,
        GetSubscriptionEstimateData,
        GetSubscriptionEstimateRequest,
        GetSubscriptionEstimateResponse,
    > for Recurly
{
}

impl api::subscriptions_v2::SubscriptionCancelV2 for Recurly {}
impl
    ConnectorIntegrationV2<
        SubscriptionCancel,
        SubscriptionCancelData,
        SubscriptionCancelRequest,
        SubscriptionCancelResponse,
    > for Recurly
{
}

impl api::subscriptions_v2::SubscriptionPauseV2 for Recurly {}
impl
    ConnectorIntegrationV2<
        SubscriptionPause,
        SubscriptionPauseData,
        SubscriptionPauseRequest,
        SubscriptionPauseResponse,
    > for Recurly
{
}

impl api::subscriptions_v2::SubscriptionResumeV2 for Recurly {}
impl
    ConnectorIntegrationV2<
        SubscriptionResume,
        SubscriptionResumeData,
        SubscriptionResumeRequest,
        SubscriptionResumeResponse,
    > for Recurly
{
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl api::revenue_recovery_v2::RevenueRecoveryRecordBackV2 for Recurly {}
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl api::revenue_recovery_v2::BillingConnectorPaymentsSyncIntegrationV2 for Recurly {}
#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl api::revenue_recovery_v2::BillingConnectorInvoiceSyncIntegrationV2 for Recurly {}

impl ConnectorCommon for Recurly {
    fn id(&self) -> &'static str {
        "recurly"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
        //    TODO! Check connector documentation, on which unit they are processing the currency.
        //    If the connector accepts amount in lower unit ( i.e cents for USD) then return api::CurrencyUnit::Minor,
        //    if connector accepts amount in base unit (i.e dollars for USD) then return api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.recurly.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = recurly::RecurlyAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![
            (
                headers::AUTHORIZATION.to_string(),
                format!(
                    "Basic {}",
                    consts::BASE64_ENGINE.encode(auth.api_key.peek())
                )
                .into_masked(),
            ),
            (
                headers::ACCEPT.to_string(),
                RECURLY_API_VERSION.to_string().into_masked(),
            ),
        ])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: recurly::RecurlyErrorResponse = res
            .response
            .parse_struct("RecurlyErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: response.reason,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
            connector_metadata: None,
        })
    }
}

impl ConnectorValidation for Recurly {
    //TODO: implement functions when support enabled
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    ConnectorIntegrationV2<
        recovery_router_flows::BillingConnectorPaymentsSync,
        recovery_flow_common_types::BillingConnectorPaymentsSyncFlowData,
        recovery_request_types::BillingConnectorPaymentsSyncRequest,
        recovery_response_types::BillingConnectorPaymentsSyncResponse,
    > for Recurly
{
    fn get_headers(
        &self,
        req: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterDataV2,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterDataV2,
    ) -> CustomResult<String, errors::ConnectorError> {
        let transaction_uuid = &req.request.billing_connector_psync_id;
        Ok(format!(
            "{}/transactions/uuid-{transaction_uuid}",
            req.request.connector_params.base_url,
        ))
    }

    fn build_request_v2(
        &self,
        req: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterDataV2,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&types::BillingConnectorPaymentsSyncTypeV2::get_url(
                self, req,
            )?)
            .attach_default_headers()
            .headers(types::BillingConnectorPaymentsSyncTypeV2::get_headers(
                self, req,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response_v2(
        &self,
        data: &recovery_router_data_types::BillingConnectorPaymentsSyncRouterDataV2,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        recovery_router_data_types::BillingConnectorPaymentsSyncRouterDataV2,
        errors::ConnectorError,
    > {
        let response: RecurlyRecoveryDetailsData = res
            .response
            .parse_struct::<RecurlyRecoveryDetailsData>("RecurlyRecoveryDetailsData")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        recovery_router_data_types::BillingConnectorPaymentsSyncRouterDataV2::try_from(
            ResponseRouterDataV2 {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
        )
    }

    fn get_error_response_v2(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    ConnectorIntegrationV2<
        InvoiceRecordBack,
        InvoiceRecordBackData,
        InvoiceRecordBackRequest,
        InvoiceRecordBackResponse,
    > for Recurly
{
    fn get_headers(
        &self,
        req: &recovery_router_data_types::InvoiceRecordBackRouterDataV2,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &recovery_router_data_types::InvoiceRecordBackRouterDataV2,
    ) -> CustomResult<String, errors::ConnectorError> {
        let invoice_id = req
            .request
            .merchant_reference_id
            .get_string_repr()
            .to_string();

        let status = RecurlyRecordStatus::try_from(req.request.attempt_status)?;

        let status_endpoint = match status {
            RecurlyRecordStatus::Success => STATUS_SUCCESSFUL_ENDPOINT,
            RecurlyRecordStatus::Failure => STATUS_FAILED_ENDPOINT,
        };

        Ok(format!(
            "{}/invoices/{invoice_id}/{status_endpoint}",
            req.request.connector_params.base_url,
        ))
    }

    fn build_request_v2(
        &self,
        req: &recovery_router_data_types::InvoiceRecordBackRouterDataV2,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Put)
                .url(&types::InvoiceRecordBackTypeV2::get_url(self, req)?)
                .attach_default_headers()
                .headers(types::InvoiceRecordBackTypeV2::get_headers(self, req)?)
                .header("Content-Length", "0")
                .build(),
        ))
    }

    fn handle_response_v2(
        &self,
        data: &recovery_router_data_types::InvoiceRecordBackRouterDataV2,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        recovery_router_data_types::InvoiceRecordBackRouterDataV2,
        errors::ConnectorError,
    > {
        let response: recurly::RecurlyRecordBackResponse = res
            .response
            .parse_struct("recurly RecurlyRecordBackResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        recovery_router_data_types::InvoiceRecordBackRouterDataV2::try_from(ResponseRouterDataV2 {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response_v2(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[cfg(all(feature = "v2", feature = "revenue_recovery"))]
impl
    ConnectorIntegrationV2<
        recovery_router_flows::BillingConnectorInvoiceSync,
        recovery_flow_common_types::BillingConnectorInvoiceSyncFlowData,
        recovery_request_types::BillingConnectorInvoiceSyncRequest,
        recovery_response_types::BillingConnectorInvoiceSyncResponse,
    > for Recurly
{
    fn get_headers(
        &self,
        req: &recovery_router_data_types::BillingConnectorInvoiceSyncRouterDataV2,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.common_get_content_type().to_string().into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &recovery_router_data_types::BillingConnectorInvoiceSyncRouterDataV2,
    ) -> CustomResult<String, errors::ConnectorError> {
        let invoice_id = &req.request.billing_connector_invoice_id;
        Ok(format!(
            "{}/invoices/{invoice_id}",
            req.request.connector_params.base_url,
        ))
    }

    fn build_request_v2(
        &self,
        req: &recovery_router_data_types::BillingConnectorInvoiceSyncRouterDataV2,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        let request = RequestBuilder::new()
            .method(Method::Get)
            .url(&types::BillingConnectorInvoiceSyncTypeV2::get_url(
                self, req,
            )?)
            .attach_default_headers()
            .headers(types::BillingConnectorInvoiceSyncTypeV2::get_headers(
                self, req,
            )?)
            .build();
        Ok(Some(request))
    }

    fn handle_response_v2(
        &self,
        data: &recovery_router_data_types::BillingConnectorInvoiceSyncRouterDataV2,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<
        recovery_router_data_types::BillingConnectorInvoiceSyncRouterDataV2,
        errors::ConnectorError,
    > {
        let response: recurly::RecurlyInvoiceSyncResponse = res
            .response
            .parse_struct::<recurly::RecurlyInvoiceSyncResponse>("RecurlyInvoiceSyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        recovery_router_data_types::BillingConnectorInvoiceSyncRouterDataV2::try_from(
            ResponseRouterDataV2 {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
        )
    }

    fn get_error_response_v2(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

#[async_trait::async_trait]
impl webhooks::IncomingWebhook for Recurly {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn common_utils::crypto::VerifySignature + Send>, errors::ConnectorError>
    {
        Ok(Box::new(common_utils::crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        // The `recurly-signature` header consists of a Unix timestamp (in milliseconds) followed by one or more HMAC-SHA256 signatures, separated by commas.
        // Multiple signatures exist when a secret key is regenerated, with the old key remaining active for 24 hours.
        let header_values = Self::get_signature_elements_from_header(request.headers)?;
        let signature = header_values
            .get(1)
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        hex::decode(signature).change_context(errors::ConnectorError::WebhookSignatureNotFound)
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let header_values = Self::get_signature_elements_from_header(request.headers)?;
        let timestamp = header_values
            .first()
            .ok_or(errors::ConnectorError::WebhookSignatureNotFound)?;
        Ok(format!(
            "{}.{}",
            String::from_utf8_lossy(timestamp),
            String::from_utf8_lossy(request.body)
        )
        .into_bytes())
    }
    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let webhook = RecurlyWebhookBody::get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(webhook.uuid),
        ))
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_object_reference_id(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    #[cfg(all(feature = "revenue_recovery", feature = "v2"))]
    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let webhook = RecurlyWebhookBody::get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        let event = match webhook.event_type {
            transformers::RecurlyPaymentEventType::PaymentSucceeded => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentSuccess
            }
            transformers::RecurlyPaymentEventType::PaymentFailed => {
                api_models::webhooks::IncomingWebhookEvent::RecoveryPaymentFailure
            }
        };
        Ok(event)
    }

    #[cfg(any(feature = "v1", not(all(feature = "revenue_recovery", feature = "v2"))))]
    fn get_webhook_event_type(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        Err(report!(errors::ConnectorError::WebhooksNotImplemented))
    }

    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let webhook = RecurlyWebhookBody::get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;
        Ok(Box::new(webhook))
    }
}

impl ConnectorSpecifications for Recurly {}
