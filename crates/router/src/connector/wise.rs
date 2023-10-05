pub mod transformers;
use std::fmt::Debug;

use error_stack::{IntoReport, ResultExt};
#[cfg(feature = "payouts")]
use masking::PeekInterface;
#[cfg(feature = "payouts")]
use router_env::{instrument, tracing};

use self::transformers as wise;
use crate::{
    configs::settings,
    core::errors::{self, CustomResult},
    headers,
    services::{
        self,
        request::{self, Mask},
        ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon, ConnectorCommonExt},
    },
    utils::BytesExt,
};
#[cfg(feature = "payouts")]
use crate::{core::payments, routes, utils};

#[derive(Debug, Clone)]
pub struct Wise;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Wise
where
    Self: services::ConnectorIntegration<Flow, Request, Response>,
{
    #[cfg(feature = "payouts")]
    fn build_headers(
        &self,
        req: &types::RouterData<Flow, Request, Response>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutQuoteType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )];
        header.append(&mut api_key);
        Ok(header)
    }
}

impl ConnectorCommon for Wise {
    fn id(&self) -> &'static str {
        "wise"
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = wise::WiseAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            auth.api_key.into_masked(),
        )])
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.wise.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: wise::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let default_status = response.status.unwrap_or_default().to_string();
        match response.errors {
            Some(errs) => {
                if let Some(e) = errs.get(0) {
                    Ok(types::ErrorResponse {
                        status_code: res.status_code,
                        code: e.code.clone(),
                        message: e.message.clone(),
                        reason: None,
                    })
                } else {
                    Ok(types::ErrorResponse {
                        status_code: res.status_code,
                        code: default_status,
                        message: response.message.unwrap_or_default(),
                        reason: None,
                    })
                }
            }
            None => Ok(types::ErrorResponse {
                status_code: res.status_code,
                code: default_status,
                message: response.message.unwrap_or_default(),
                reason: None,
            }),
        }
    }
}

impl api::Payment for Wise {}
impl api::PaymentAuthorize for Wise {}
impl api::PaymentSync for Wise {}
impl api::PaymentVoid for Wise {}
impl api::PaymentCapture for Wise {}
impl api::MandateSetup for Wise {}
impl api::ConnectorAccessToken for Wise {}
impl api::PaymentToken for Wise {}
impl ConnectorValidation for Wise {}

impl
    services::ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Wise
{
}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Wise
{
}

impl
    services::ConnectorIntegration<
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Wise
{
}

impl api::PaymentSession for Wise {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Wise
{
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Wise
{
}

impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Wise
{
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Wise
{
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Wise
{
}

impl api::Payouts for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutCancel for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutCreate for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutEligibility for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutQuote for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutRecipient for Wise {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Wise {}

#[cfg(feature = "payouts")]
impl services::ConnectorIntegration<api::PoCancel, types::PayoutsData, types::PayoutsResponseData>
    for Wise
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let transfer_id = req.request.connector_payout_id.clone().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "transfer_id",
            },
        )?;
        Ok(format!(
            "{}v1/transfers/{}/cancel",
            connectors.wise.base_url, transfer_id
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, _connectors)
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Put)
            .url(&types::PayoutCancelType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutCancelType::get_headers(self, req, connectors)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoCancel>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoCancel>, errors::ConnectorError> {
        let response: wise::WisePayoutResponse = res
            .response
            .parse_struct("WisePayoutResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: wise::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let def_res = response.status.unwrap_or_default().to_string();
        let errors = response.errors.unwrap_or_default();
        let (code, message) = if let Some(e) = errors.get(0) {
            (e.code.clone(), e.message.clone())
        } else {
            (def_res, response.message.unwrap_or_default())
        };
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code,
            message,
            reason: None,
        })
    }
}

#[cfg(feature = "payouts")]
impl services::ConnectorIntegration<api::PoQuote, types::PayoutsData, types::PayoutsResponseData>
    for Wise
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoQuote>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(format!(
            "{}v3/profiles/{}/quotes",
            connectors.wise.base_url,
            auth.profile_id.peek()
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoQuote>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoQuote>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = wise::WisePayoutQuoteRequest::try_from(req)?;
        let wise_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<wise::WisePayoutQuoteRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(wise_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoQuote>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutQuoteType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutQuoteType::get_headers(self, req, connectors)?)
            .body(types::PayoutQuoteType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoQuote>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoQuote>, errors::ConnectorError> {
        let response: wise::WisePayoutQuoteResponse = res
            .response
            .parse_struct("WisePayoutQuoteResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[cfg(feature = "payouts")]
impl
    services::ConnectorIntegration<api::PoRecipient, types::PayoutsData, types::PayoutsResponseData>
    for Wise
{
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoRecipient>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v1/accounts", connectors.wise.base_url))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoRecipient>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoRecipient>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = wise::WiseRecipientCreateRequest::try_from(req)?;
        let wise_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<wise::WiseRecipientCreateRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(wise_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoRecipient>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutRecipientType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutRecipientType::get_headers(
                self, req, connectors,
            )?)
            .body(types::PayoutRecipientType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoRecipient>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoRecipient>, errors::ConnectorError> {
        let response: wise::WiseRecipientCreateResponse = res
            .response
            .parse_struct("WiseRecipientCreateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[async_trait::async_trait]
#[cfg(feature = "payouts")]
impl services::ConnectorIntegration<api::PoCreate, types::PayoutsData, types::PayoutsResponseData>
    for Wise
{
    async fn execute_pretasks(
        &self,
        router_data: &mut types::PayoutsRouterData<api::PoCreate>,
        app_state: &routes::AppState,
    ) -> CustomResult<(), errors::ConnectorError> {
        // Create a quote
        let quote_router_data =
            &types::PayoutsRouterData::from((&router_data, router_data.request.clone()));
        let quote_connector_integration: Box<
            &(dyn services::ConnectorIntegration<
                api::PoQuote,
                types::PayoutsData,
                types::PayoutsResponseData,
            > + Send
                  + Sync
                  + 'static),
        > = Box::new(self);
        let quote_router_resp = services::execute_connector_processing_step(
            app_state,
            quote_connector_integration,
            quote_router_data,
            payments::CallConnectorAction::Trigger,
            None,
        )
        .await?;

        match quote_router_resp.response.to_owned() {
            Ok(resp) => {
                router_data.quote_id = Some(resp.connector_payout_id);
                Ok(())
            }
            Err(_err) => {
                router_data.response = quote_router_resp.response;
                Ok(())
            }
        }
    }

    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v1/transfers", connectors.wise.base_url))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = wise::WisePayoutCreateRequest::try_from(req)?;
        let wise_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<wise::WisePayoutCreateRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(wise_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutCreateType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutCreateType::get_headers(self, req, connectors)?)
            .body(types::PayoutCreateType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoCreate>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoCreate>, errors::ConnectorError> {
        let response: wise::WisePayoutResponse = res
            .response
            .parse_struct("WisePayoutResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

#[cfg(feature = "payouts")]
impl
    services::ConnectorIntegration<
        api::PoEligibility,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for Wise
{
}

#[cfg(feature = "payouts")]
impl services::ConnectorIntegration<api::PoFulfill, types::PayoutsData, types::PayoutsResponseData>
    for Wise
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let transfer_id = req.request.connector_payout_id.to_owned().ok_or(
            errors::ConnectorError::MissingRequiredField {
                field_name: "transfer_id",
            },
        )?;
        Ok(format!(
            "{}v3/profiles/{}/transfers/{}/payments",
            connectors.wise.base_url,
            auth.profile_id.peek(),
            transfer_id
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = wise::WisePayoutFulfillRequest::try_from(req)?;
        let wise_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<wise::WisePayoutFulfillRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(wise_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutFulfillType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutFulfillType::get_headers(
                self, req, connectors,
            )?)
            .body(types::PayoutFulfillType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoFulfill>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoFulfill>, errors::ConnectorError> {
        let response: wise::WiseFulfillResponse = res
            .response
            .parse_struct("WiseFulfillResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl api::Refund for Wise {}
impl api::RefundExecute for Wise {}
impl api::RefundSync for Wise {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Wise
{
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Wise
{
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Wise {
    fn get_webhook_object_reference_id(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_event_type(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::IncomingWebhookEvent, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }

    fn get_webhook_resource_object(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        Err(errors::ConnectorError::WebhooksNotImplemented).into_report()
    }
}
