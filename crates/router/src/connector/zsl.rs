pub mod transformers;

use std::fmt::Debug;

use common_utils::ext_traits::ValueExt;
use diesel_models::enums;
use error_stack::ResultExt;
use masking::ExposeInterface;
use transformers as zsl;

use crate::{
    configs::settings,
    connector::utils as connector_utils,
    core::errors::{self, CustomResult},
    events::connector_api_logs::ConnectorEvent,
    headers,
    services::{
        self,
        request::{self},
        ConnectorIntegration, ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
        transformers::ForeignFrom,
        ErrorResponse, RequestContent, Response,
    },
    utils::BytesExt,
};

#[derive(Debug, Clone)]
pub struct Zsl;

impl api::Payment for Zsl {}
impl api::PaymentSession for Zsl {}
impl api::ConnectorAccessToken for Zsl {}
impl api::MandateSetup for Zsl {}
impl api::PaymentAuthorize for Zsl {}
impl api::PaymentSync for Zsl {}
impl api::PaymentCapture for Zsl {}
impl api::PaymentVoid for Zsl {}
impl api::Refund for Zsl {}
impl api::RefundExecute for Zsl {}
impl api::RefundSync for Zsl {}
impl api::PaymentToken for Zsl {}

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Zsl
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
    fn build_headers(
        &self,
        _req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let header = vec![(
            headers::CONTENT_TYPE.to_string(),
            self.get_content_type().to_string().into(),
        )];
        Ok(header)
    }
}

impl ConnectorCommon for Zsl {
    fn id(&self) -> &'static str {
        "zsl"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/x-www-form-urlencoded"
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.zsl.base_url.as_ref()
    }

    fn get_auth_header(
        &self,
        _auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        Ok(vec![])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response = serde_urlencoded::from_bytes::<zsl::ZslErrorResponse>(&res.response)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        let error_reason = zsl::ZslResponseStatus::try_from(response.status.clone())?.to_string();

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.status,
            message: error_reason.clone(),
            reason: Some(error_reason),
            attempt_status: Some(common_enums::AttemptStatus::Failure),
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Zsl {
    fn validate_capture_method(
        &self,
        capture_method: Option<enums::CaptureMethod>,
        _pmt: Option<enums::PaymentMethodType>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic => Ok(()),
            enums::CaptureMethod::Manual
            | enums::CaptureMethod::ManualMultiple
            | enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_supported_error_report(capture_method, self.id()),
            ),
        }
    }
}

impl ConnectorIntegration<api::Authorize, types::PaymentsAuthorizeData, types::PaymentsResponseData>
    for Zsl
{
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}ecp", self.base_url(connectors).to_string()))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = zsl::ZslRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = zsl::ZslPaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::FormUrlEncoded(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsAuthorizeType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response = serde_urlencoded::from_bytes::<zsl::ZslPaymentsResponse>(&res.response)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i: &mut ConnectorEvent| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }

    fn get_5xx_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

impl ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Zsl
{
    fn build_request(
        &self,
        _req: &types::PaymentsSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Psync".to_owned(),
            connector: "Zsl",
        }
        .into())
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: zsl::ZslWebhookResponse = res
            .response
            .parse_struct("ZslWebhookResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i: &mut ConnectorEvent| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
}

impl ConnectorIntegration<api::Session, types::PaymentsSessionData, types::PaymentsResponseData>
    for Zsl
{
    fn build_request(
        &self,
        _req: &types::PaymentsSessionRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Session flow".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl
    ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Zsl
{
    fn build_request(
        &self,
        _req: &types::TokenizationRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "PaymentMethod Tokenization flow ".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<api::AccessTokenAuth, types::AccessTokenRequestData, types::AccessToken>
    for Zsl
{
    fn build_request(
        &self,
        _req: &types::RefreshTokenRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "AccessTokenAuth flow".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl
    ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Zsl
{
    fn build_request(
        &self,
        _req: &types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "SetupMandate flow".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<api::Capture, types::PaymentsCaptureData, types::PaymentsResponseData>
    for Zsl
{
    fn build_request(
        &self,
        _req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Capture flow".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<api::Void, types::PaymentsCancelData, types::PaymentsResponseData>
    for Zsl
{
    fn build_request(
        &self,
        _req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Void flow ".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData> for Zsl {
    fn build_request(
        &self,
        _req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Refund flow".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

impl ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData> for Zsl {
    fn build_request(
        &self,
        _req: &types::RefundSyncRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Err(errors::ConnectorError::NotSupported {
            message: "Rsync flow ".to_owned(),
            connector: "Zsl",
        }
        .into())
    }
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Zsl {
    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::PaymentAttemptId(notif.mer_ref),
        ))
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        Ok(api_models::webhooks::IncomingWebhookEvent::foreign_from(
            notif.status,
        ))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let response = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(Box::new(response))
    }

    async fn verify_webhook_source(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_account: &types::domain::MerchantAccount,
        merchant_connector_account: types::domain::MerchantConnectorAccount,
        _connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_account_details = merchant_connector_account
            .connector_account_details
            .parse_value::<types::ConnectorAuthType>("ConnectorAuthType")
            .change_context_lazy(|| errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let auth_type = zsl::ZslAuthType::try_from(&connector_account_details)?;
        let key = auth_type.api_key.expose();
        let mer_id = auth_type.merchant_id.expose();
        let webhook_response = get_webhook_object_from_body(request.body)?;
        let signature = zsl::calculate_signature(
            webhook_response.enctype,
            zsl::ZslSignatureType::WebhookSignature {
                status: webhook_response.status,
                txn_id: webhook_response.txn_id,
                txn_date: webhook_response.txn_date,
                paid_ccy: webhook_response.paid_ccy.to_string(),
                paid_amt: webhook_response.paid_amt,
                mer_ref: webhook_response.mer_ref,
                mer_id,
                key,
            },
        )?;

        Ok(signature.eq(&webhook_response.signature))
    }

    fn get_webhook_api_response(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ConnectorError>
    {
        Ok(services::api::ApplicationResponse::TextPlain(
            "CALLBACK-OK".to_string(),
        ))
    }
}

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<zsl::ZslWebhookResponse, errors::ConnectorError> {
    let response: zsl::ZslWebhookResponse =
        serde_urlencoded::from_bytes::<zsl::ZslWebhookResponse>(body)
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
    Ok(response)
}
