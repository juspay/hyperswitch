mod transformers;

use std::fmt::Debug;

use error_stack::{IntoReport, ResultExt};
use router_env::{instrument, tracing};

use self::transformers as wise;
use crate::{
    configs::settings,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers, logger, routes, services,
    types::{
        self,
        api::{self, ConnectorCommon},
    },
    utils::{self, BytesExt},
};

#[derive(Debug, Clone)]
pub struct Wise;

impl ConnectorCommon for Wise {
    fn id(&self) -> &'static str {
        "wise"
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let auth = wise::WiseAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key)])
    }

    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.wise.base_url.as_ref()
    }
}

impl api::Payment for Wise {}
impl api::PaymentAuthorize for Wise {}
impl api::PaymentSync for Wise {}
impl api::PaymentVoid for Wise {}
impl api::PaymentCapture for Wise {}
impl api::PreVerify for Wise {}
impl api::ConnectorAccessToken for Wise {}
impl api::PaymentToken for Wise {}

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
        api::Verify,
        types::VerifyRequestData,
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
impl api::PayoutCancel for Wise {}
impl api::PayoutCreate for Wise {}
impl api::PayoutEligibility for Wise {}
impl api::PayoutQuote for Wise {}
impl api::PayoutRecipient for Wise {}
impl api::PayoutFulfill for Wise {}

impl services::ConnectorIntegration<api::PCancel, types::PayoutsData, types::PayoutsResponseData>
    for Wise
{
}

impl services::ConnectorIntegration<api::PQuote, types::PayoutsData, types::PayoutsResponseData>
    for Wise
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PQuote>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(format!(
            "{}v3/profiles/{}/quotes",
            connectors.wise.base_url, auth.profile_id
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PQuote>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutQuoteType::get_content_type(self).to_string(),
        )];
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(headers::AUTHORIZATION.to_string(), auth.api_key)];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PQuote>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let connector_req = wise::WisePayoutQuoteRequest::try_from(req)?;
        let wise_req =
            utils::Encode::<wise::WisePayoutQuoteRequest>::encode_to_string_of_json(&connector_req)
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(wise_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PQuote>,
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
        data: &types::PayoutsRouterData<api::PQuote>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PQuote>, errors::ConnectorError> {
        let response: wise::WisePayoutQuoteResponse = res
            .response
            .parse_struct("WisePayoutQuoteResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: wise::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);
        let def_res = response.status.unwrap_or_default().to_string();
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
                        code: def_res,
                        message: response.message.unwrap_or_default(),
                        reason: None,
                    })
                }
            }
            None => Ok(types::ErrorResponse {
                status_code: res.status_code,
                code: def_res,
                message: response.message.unwrap_or_default(),
                reason: None,
            }),
        }
    }
}

impl services::ConnectorIntegration<api::PRecipient, types::PayoutsData, types::PayoutsResponseData>
    for Wise
{
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PRecipient>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v1/accounts", connectors.wise.base_url))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PRecipient>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutRecipientType::get_content_type(self).to_string(),
        )];
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(headers::AUTHORIZATION.to_string(), auth.api_key)];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PRecipient>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let connector_req = wise::WiseRecipientCreateRequest::try_from(req)?;
        let wise_req = utils::Encode::<wise::WiseRecipientCreateRequest>::encode_to_string_of_json(
            &connector_req,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(wise_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PRecipient>,
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
        data: &types::PayoutsRouterData<api::PRecipient>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PRecipient>, errors::ConnectorError> {
        let response: wise::WiseRecipientCreateResponse = res
            .response
            .parse_struct("WiseRecipientCreateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: wise::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);
        let def_res = response.status.unwrap_or_default().to_string();
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
                        code: def_res,
                        message: response.message.unwrap_or_default(),
                        reason: None,
                    })
                }
            }
            None => Ok(types::ErrorResponse {
                status_code: res.status_code,
                code: def_res,
                message: response.message.unwrap_or_default(),
                reason: None,
            }),
        }
    }
}

#[async_trait::async_trait]
impl services::ConnectorIntegration<api::PCreate, types::PayoutsData, types::PayoutsResponseData>
    for Wise
{
    async fn execute_pretasks(
        &self,
        router_data: &mut types::PayoutsRouterData<api::PCreate>,
        app_state: &routes::AppState,
    ) -> CustomResult<(), errors::ConnectorError> {
        // Create a quote
        let quote_router_data =
            &types::PayoutsRouterData::from((&router_data, router_data.request.clone()));
        let quote_connector_integration: Box<
            &(dyn services::ConnectorIntegration<
                api::PQuote,
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
        )
        .await?;
        if let Ok(resp) = quote_router_resp.response {
            router_data.request.quote_id = Some(resp.connector_payout_id);
        };
        Ok(())
    }

    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/v1/transfers", connectors.wise.base_url))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PCreate>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutCreateType::get_content_type(self).to_string(),
        )];
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(headers::AUTHORIZATION.to_string(), auth.api_key)];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PCreate>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let connector_req = wise::WisePayoutCreateRequest::try_from(req)?;
        let wise_req = utils::Encode::<wise::WisePayoutCreateRequest>::encode_to_string_of_json(
            &connector_req,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(wise_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PCreate>,
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
        data: &types::PayoutsRouterData<api::PCreate>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PCreate>, errors::ConnectorError> {
        let response: wise::WisePayoutCreateResponse = res
            .response
            .parse_struct("WisePayoutCreateResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: wise::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);
        let def_res = response.status.unwrap_or_default().to_string();
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
                        code: def_res,
                        message: response.message.unwrap_or_default(),
                        reason: None,
                    })
                }
            }
            None => Ok(types::ErrorResponse {
                status_code: res.status_code,
                code: def_res,
                message: response.message.unwrap_or_default(),
                reason: None,
            }),
        }
    }
}

impl
    services::ConnectorIntegration<
        api::PEligibility,
        types::PayoutsData,
        types::PayoutsResponseData,
    > for Wise
{
}

impl services::ConnectorIntegration<api::PFulfill, types::PayoutsData, types::PayoutsResponseData>
    for Wise
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PFulfill>,
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
            connectors.wise.base_url, auth.profile_id, transfer_id
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, String)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutFulfillType::get_content_type(self).to_string(),
        )];
        let auth = wise::WiseAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(headers::AUTHORIZATION.to_string(), auth.api_key)];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PFulfill>,
    ) -> CustomResult<Option<String>, errors::ConnectorError> {
        let connector_req = wise::WisePayoutFulfillRequest::try_from(req)?;
        let wise_req = utils::Encode::<wise::WisePayoutFulfillRequest>::encode_to_string_of_json(
            &connector_req,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(wise_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PFulfill>,
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
        data: &types::PayoutsRouterData<api::PFulfill>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PFulfill>, errors::ConnectorError> {
        let response: wise::WiseFulfillResponse = res
            .response
            .parse_struct("WiseFulfillResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);
        types::RouterData::try_from(types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: wise::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        logger::info!(response=?res);
        let def_res = response.status.unwrap_or_default().to_string();
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
                        code: def_res,
                        message: response.message.unwrap_or_default(),
                        reason: None,
                    })
                }
            }
            None => Ok(types::ErrorResponse {
                status_code: res.status_code,
                code: def_res,
                message: response.message.unwrap_or_default(),
                reason: None,
            }),
        }
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
