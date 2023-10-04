pub mod transformers;

use std::fmt::Debug;

use api_models::webhooks::IncomingWebhookEvent;
use base64::Engine;
use diesel_models::{enums as storage_enums, enums};
use error_stack::{IntoReport, ResultExt};
use ring::hmac;
use router_env::{instrument, tracing};

use self::transformers as adyen;
use crate::{
    configs::settings,
    connector::utils as connector_utils,
    consts,
    core::{
        self,
        errors::{self, CustomResult},
    },
    headers, logger, routes,
    services::{
        self,
        request::{self, Mask},
        ConnectorValidation,
    },
    types::{
        self,
        api::{self, ConnectorCommon},
        domain,
        transformers::ForeignFrom,
    },
    utils::{self, crypto, ByteSliceExt, BytesExt, OptionExt},
};

#[derive(Debug, Clone)]
pub struct Adyen;

impl ConnectorCommon for Adyen {
    fn id(&self) -> &'static str {
        "adyen"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

    fn get_auth_header(
        &self,
        auth_type: &types::ConnectorAuthType,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let auth = adyen::AdyenAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(
            headers::X_API_KEY.to_string(),
            auth.api_key.into_masked(),
        )])
    }
    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.adyen.base_url.as_ref()
    }

    fn build_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: adyen::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.message,
            reason: None,
        })
    }
}

impl ConnectorValidation for Adyen {
    fn validate_capture_method(
        &self,
        capture_method: Option<storage_enums::CaptureMethod>,
    ) -> CustomResult<(), errors::ConnectorError> {
        let capture_method = capture_method.unwrap_or_default();
        match capture_method {
            enums::CaptureMethod::Automatic
            | enums::CaptureMethod::Manual
            | enums::CaptureMethod::ManualMultiple => Ok(()),
            enums::CaptureMethod::Scheduled => Err(
                connector_utils::construct_not_implemented_error_report(capture_method, self.id()),
            ),
        }
    }
    fn validate_psync_reference_id(
        &self,
        data: &types::PaymentsSyncRouterData,
    ) -> CustomResult<(), errors::ConnectorError> {
        if data.request.encoded_data.is_some() {
            return Ok(());
        }
        Err(errors::ConnectorError::MissingRequiredField {
            field_name: "encoded_data",
        }
        .into())
    }
    fn is_webhook_source_verification_mandatory(&self) -> bool {
        true
    }
}

impl api::Payment for Adyen {}
impl api::PaymentAuthorize for Adyen {}
impl api::PaymentSync for Adyen {}
impl api::PaymentVoid for Adyen {}
impl api::PaymentCapture for Adyen {}
impl api::PreVerify for Adyen {}
impl api::ConnectorAccessToken for Adyen {}
impl api::PaymentToken for Adyen {}

impl
    services::ConnectorIntegration<
        api::PaymentMethodToken,
        types::PaymentMethodTokenizationData,
        types::PaymentsResponseData,
    > for Adyen
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::AccessTokenAuth,
        types::AccessTokenRequestData,
        types::AccessToken,
    > for Adyen
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Verify,
        types::VerifyRequestData,
        types::PaymentsResponseData,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::VerifyRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsVerifyType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        _req: &types::VerifyRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "v68/payments"))
    }
    fn get_request_body(
        &self,
        req: &types::VerifyRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let authorize_req = types::PaymentsAuthorizeRouterData::from((
            req,
            types::PaymentsAuthorizeData::from(req),
        ));
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            authorize_req.request.currency,
            authorize_req.request.amount,
            &authorize_req,
        ))?;
        let connector_req = adyen::AdyenPaymentRequest::try_from(&connector_router_data)?;

        let adyen_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<adyen::AdyenPaymentRequest<'_>>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(adyen_req))
    }
    fn build_request(
        &self,
        req: &types::VerifyRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVerifyType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVerifyType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsVerifyType::get_request_body(self, req)?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::VerifyRouterData,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<api::Verify, types::VerifyRequestData, types::PaymentsResponseData>,
        errors::ConnectorError,
    >
    where
        api::Verify: Clone,
        types::VerifyRequestData: Clone,
        types::PaymentsResponseData: Clone,
    {
        let response: adyen::AdyenPaymentResponse = res
            .response
            .parse_struct("AdyenPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            None,
            false,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: adyen::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.message,
            reason: None,
        })
    }
}

impl api::PaymentSession for Adyen {}

impl
    services::ConnectorIntegration<
        api::Session,
        types::PaymentsSessionData,
        types::PaymentsResponseData,
    > for Adyen
{
    // Not Implemented (R)
}

impl
    services::ConnectorIntegration<
        api::Capture,
        types::PaymentsCaptureData,
        types::PaymentsResponseData,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
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
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.as_str();
        Ok(format!(
            "{}{}/{}/captures",
            self.base_url(connectors),
            "v68/payments",
            id
        ))
    }
    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let connector_req = adyen::AdyenCaptureRequest::try_from(&connector_router_data)?;
        let adyen_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<adyen::AdyenCaptureRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(adyen_req))
    }
    fn build_request(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsCaptureType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsCaptureType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsCaptureType::get_request_body(self, req)?)
                .build(),
        ))
    }
    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: adyen::AdyenCaptureResponse = res
            .response
            .parse_struct("AdyenCaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

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
        let response: adyen::ErrorResponse = res
            .response
            .parse_struct("adyen::ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.message,
            reason: None,
        })
    }
}

/// Payment Sync can be useful only incase of Redirect flow.
/// For payments which doesn't involve redrection we have to rely on webhooks.
impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Adyen
{
    fn get_headers(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsSyncType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        // Adyen doesn't support PSync flow. We use PSync flow to fetch payment details,
        // specifically the redirect URL that takes the user to their Payment page. In non-redirection flows,
        // we rely on webhooks to obtain the payment status since there is no encoded data available.
        // encoded_data only includes the redirect URL and is only relevant in redirection flows.
        let encoded_value = req
            .request
            .encoded_data
            .clone()
            .get_required_value("encoded_data");

        match encoded_value {
            Ok(encoded_data) => {
                let adyen_redirection_type = serde_urlencoded::from_str::<
                    transformers::AdyenRedirectRequestTypes,
                >(encoded_data.as_str())
                .into_report()
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

                let redirection_request = match adyen_redirection_type {
                    adyen::AdyenRedirectRequestTypes::AdyenRedirection(req) => {
                        adyen::AdyenRedirectRequest {
                            details: adyen::AdyenRedirectRequestTypes::AdyenRedirection(
                                adyen::AdyenRedirection {
                                    redirect_result: req.redirect_result,
                                    type_of_redirection_result: None,
                                    result_code: None,
                                },
                            ),
                        }
                    }
                    adyen::AdyenRedirectRequestTypes::AdyenThreeDS(req) => {
                        adyen::AdyenRedirectRequest {
                            details: adyen::AdyenRedirectRequestTypes::AdyenThreeDS(
                                adyen::AdyenThreeDS {
                                    three_ds_result: req.three_ds_result,
                                    type_of_redirection_result: None,
                                    result_code: None,
                                },
                            ),
                        }
                    }
                    adyen::AdyenRedirectRequestTypes::AdyenRefusal(req) => {
                        adyen::AdyenRedirectRequest {
                            details: adyen::AdyenRedirectRequestTypes::AdyenRefusal(
                                adyen::AdyenRefusal {
                                    payload: req.payload,
                                    type_of_redirection_result: None,
                                    result_code: None,
                                },
                            ),
                        }
                    }
                };

                let adyen_request = types::RequestBody::log_and_get_request_body(
                    &redirection_request,
                    utils::Encode::<adyen::AdyenRedirectRequest>::encode_to_string_of_json,
                )
                .change_context(errors::ConnectorError::RequestEncodingFailed)?;

                Ok(Some(adyen_request))
            }
            Err(_) => Ok(None),
        }
    }

    fn get_url(
        &self,
        _req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "v68/payments/details"
        ))
    }

    fn build_request(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request_body = self.get_request_body(req)?;
        match request_body {
            Some(_) => Ok(Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                    .body(types::PaymentsSyncType::get_request_body(self, req)?)
                    .build(),
            )),
            None => Ok(None),
        }
    }

    fn handle_response(
        &self,
        data: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        res: types::Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        logger::debug!(payment_sync_response=?res);
        let response: adyen::AdyenPaymentResponse = res
            .response
            .parse_struct("AdyenPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        let is_multiple_capture_sync = match data.request.sync_type {
            types::SyncRequestType::MultipleCaptureSync(_) => true,
            types::SyncRequestType::SinglePaymentSync => false,
        };
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            data.request.capture_method,
            is_multiple_capture_sync,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: adyen::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.message,
            reason: None,
        })
    }

    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<services::CaptureSyncMethod, errors::ConnectorError> {
        Ok(services::CaptureSyncMethod::Individual)
    }
}

#[async_trait::async_trait]
impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Adyen
{
    async fn execute_pretasks(
        &self,
        router_data: &mut types::PaymentsAuthorizeRouterData,
        app_state: &routes::AppState,
    ) -> CustomResult<(), errors::ConnectorError> {
        match &router_data.request.payment_method_data {
            api_models::payments::PaymentMethodData::GiftCard(gift_card_data) => {
                match gift_card_data.as_ref() {
                    api_models::payments::GiftCardData::Givex(_) => {
                        let integ: Box<
                            &(dyn services::ConnectorIntegration<
                                api::Balance,
                                types::PaymentsAuthorizeData,
                                types::PaymentsResponseData,
                            > + Send
                                  + Sync
                                  + 'static),
                        > = Box::new(&Self);

                        let authorize_data = &types::PaymentsBalanceRouterData::from((
                            &router_data.to_owned(),
                            router_data.request.clone(),
                        ));

                        let resp = services::execute_connector_processing_step(
                            app_state,
                            integ,
                            authorize_data,
                            core::payments::CallConnectorAction::Trigger,
                            None,
                        )
                        .await?;
                        router_data.payment_method_balance = resp.payment_method_balance;

                        Ok(())
                    }
                    _ => Ok(()),
                }
            }
            _ => Ok(()),
        }
    }

    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError>
    where
        Self: services::ConnectorIntegration<
            api::Authorize,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "v68/payments"))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = adyen::AdyenPaymentRequest::try_from(&connector_router_data)?;
        let request_body = types::RequestBody::log_and_get_request_body(
        &connector_req,
        common_utils::ext_traits::Encode::<adyen::AdyenPaymentRequest<'_>>::encode_to_string_of_json,
    )
    .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(request_body))
    }

    fn build_request(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        check_for_payment_method_balance(req)?;
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
                .body(types::PaymentsAuthorizeType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: adyen::AdyenPaymentResponse = res
            .response
            .parse_struct("AdyenPaymentResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        types::RouterData::try_from((
            types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            },
            data.request.capture_method,
            false,
        ))
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: types::Response,
    ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
        let response: adyen::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.message,
            reason: None,
        })
    }
}

impl
    services::ConnectorIntegration<
        api::Balance,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::PaymentsBalanceRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError>
    where
        Self: services::ConnectorIntegration<
            api::Balance,
            types::PaymentsAuthorizeData,
            types::PaymentsResponseData,
        >,
    {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsBalanceType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        _req: &types::PaymentsBalanceRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}v69/paymentMethods/balance",
            self.base_url(connectors)
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsBalanceRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = adyen::AdyenBalanceRequest::try_from(req)?;

        let adyen_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<adyen::AdyenBalanceRequest<'_>>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(adyen_req))
    }

    fn build_request(
        &self,
        req: &types::PaymentsBalanceRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsBalanceType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsBalanceType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::PaymentsBalanceType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsBalanceRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsBalanceRouterData, errors::ConnectorError> {
        let response: adyen::AdyenBalanceResponse = res
            .response
            .parse_struct("AdyenBalanceResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
        self.build_error_response(res)
    }
}

impl
    services::ConnectorIntegration<
        api::Void,
        types::PaymentsCancelData,
        types::PaymentsResponseData,
    > for Adyen
{
    fn get_headers(
        &self,
        req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsAuthorizeType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let id = req.request.connector_transaction_id.as_str();
        Ok(format!(
            "{}v68/payments/{}/cancels",
            self.base_url(connectors),
            id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = adyen::AdyenCancelRequest::try_from(req)?;

        let adyen_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<adyen::AdyenCancelRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(adyen_req))
    }
    fn build_request(
        &self,
        req: &types::PaymentsCancelRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsVoidType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::PaymentsVoidType::get_headers(self, req, connectors)?)
                .body(types::PaymentsVoidType::get_request_body(self, req)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCancelRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsCancelRouterData, errors::ConnectorError> {
        let response: adyen::AdyenCancelResponse = res
            .response
            .parse_struct("AdyenCancelResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
        let response: adyen::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.message,
            reason: None,
        })
    }
}

impl api::Payouts for Adyen {}
#[cfg(feature = "payouts")]
impl api::PayoutCancel for Adyen {}
#[cfg(feature = "payouts")]
impl api::PayoutCreate for Adyen {}
#[cfg(feature = "payouts")]
impl api::PayoutEligibility for Adyen {}
#[cfg(feature = "payouts")]
impl api::PayoutFulfill for Adyen {}

#[cfg(feature = "payouts")]
impl services::ConnectorIntegration<api::PoCancel, types::PayoutsData, types::PayoutsResponseData>
    for Adyen
{
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoCancel>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}pal/servlet/Payout/v68/declineThirdParty",
            connectors.adyen.secondary_base_url
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutCancelType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let auth = adyen::AdyenAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(
            headers::X_API_KEY.to_string(),
            auth.review_key.unwrap_or(auth.api_key).into_masked(),
        )];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_req = adyen::AdyenPayoutCancelRequest::try_from(req)?;
        let adyen_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<adyen::AdyenPayoutCancelRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(adyen_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutCancelType::get_url(self, req, connectors)?)
            .attach_default_headers()
            .headers(types::PayoutCancelType::get_headers(self, req, connectors)?)
            .body(types::PayoutCancelType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoCancel>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoCancel>, errors::ConnectorError> {
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
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
impl services::ConnectorIntegration<api::PoCreate, types::PayoutsData, types::PayoutsResponseData>
    for Adyen
{
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoCreate>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}pal/servlet/Payout/v68/storeDetailAndSubmitThirdParty",
            connectors.adyen.secondary_base_url
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutCreateType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.destination_currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = adyen::AdyenPayoutCreateRequest::try_from(&connector_router_data)?;
        let adyen_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<adyen::AdyenPayoutCreateRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(adyen_req))
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
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
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
    > for Adyen
{
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoEligibility>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v68/payments", self.base_url(connectors),))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoEligibility>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutEligibilityType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoEligibility>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.destination_currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = adyen::AdyenPayoutEligibilityRequest::try_from(&connector_router_data)?;
        let adyen_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<adyen::AdyenPayoutEligibilityRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(adyen_req))
    }

    fn build_request(
        &self,
        req: &types::PayoutsRouterData<api::PoEligibility>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        let request = services::RequestBuilder::new()
            .method(services::Method::Post)
            .url(&types::PayoutEligibilityType::get_url(
                self, req, connectors,
            )?)
            .attach_default_headers()
            .headers(types::PayoutEligibilityType::get_headers(
                self, req, connectors,
            )?)
            .body(types::PayoutEligibilityType::get_request_body(self, req)?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::PayoutsRouterData<api::PoEligibility>,
        res: types::Response,
    ) -> CustomResult<types::PayoutsRouterData<api::PoEligibility>, errors::ConnectorError> {
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
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
impl services::ConnectorIntegration<api::PoFulfill, types::PayoutsData, types::PayoutsResponseData>
    for Adyen
{
    fn get_url(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}pal/servlet/Payout/v68/{}",
            connectors.adyen.secondary_base_url,
            match req.request.payout_type {
                storage_enums::PayoutType::Bank => "confirmThirdParty".to_string(),
                storage_enums::PayoutType::Card => "payout".to_string(),
            }
        ))
    }

    fn get_headers(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PayoutFulfillType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let auth = adyen::AdyenAuthType::try_from(&req.connector_auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let mut api_key = vec![(
            headers::X_API_KEY.to_string(),
            match req.request.payout_type {
                storage_enums::PayoutType::Bank => {
                    auth.review_key.unwrap_or(auth.api_key).into_masked()
                }
                storage_enums::PayoutType::Card => auth.api_key.into_masked(),
            },
        )];
        header.append(&mut api_key);
        Ok(header)
    }

    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.destination_currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = adyen::AdyenPayoutFulfillRequest::try_from(&connector_router_data)?;
        let adyen_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<adyen::AdyenPayoutFulfillRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(adyen_req))
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
        let response: adyen::AdyenPayoutResponse = res
            .response
            .parse_struct("AdyenPayoutResponse")
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

impl api::Refund for Adyen {}
impl api::RefundExecute for Adyen {}
impl api::RefundSync for Adyen {}

impl services::ConnectorIntegration<api::Execute, types::RefundsData, types::RefundsResponseData>
    for Adyen
{
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::RefundExecuteType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_header = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_header);
        Ok(header)
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let connector_payment_id = req.request.connector_transaction_id.clone();
        Ok(format!(
            "{}v68/payments/{}/refunds",
            self.base_url(connectors),
            connector_payment_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
    ) -> CustomResult<Option<types::RequestBody>, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req = adyen::AdyenRefundRequest::try_from(&connector_router_data)?;

        let adyen_req = types::RequestBody::log_and_get_request_body(
            &connector_req,
            utils::Encode::<adyen::AdyenRefundRequest>::encode_to_string_of_json,
        )
        .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        Ok(Some(adyen_req))
    }

    fn build_request(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::RefundExecuteType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::RefundExecuteType::get_headers(
                    self, req, connectors,
                )?)
                .body(types::RefundExecuteType::get_request_body(self, req)?)
                .build(),
        ))
    }

    #[instrument(skip_all)]
    fn handle_response(
        &self,
        data: &types::RefundsRouterData<api::Execute>,
        res: types::Response,
    ) -> CustomResult<types::RefundsRouterData<api::Execute>, errors::ConnectorError> {
        let response: adyen::AdyenRefundResponse = res
            .response
            .parse_struct("AdyenRefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
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
        let response: adyen::ErrorResponse = res
            .response
            .parse_struct("ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(types::ErrorResponse {
            status_code: res.status_code,
            code: response.error_code,
            message: response.message,
            reason: None,
        })
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Adyen
{
}

fn get_webhook_object_from_body(
    body: &[u8],
) -> CustomResult<adyen::AdyenNotificationRequestItemWH, errors::ParsingError> {
    let mut webhook: adyen::AdyenIncomingWebhook = body.parse_struct("AdyenIncomingWebhook")?;

    let item_object = webhook
        .notification_items
        .drain(..)
        .next()
        // TODO: ParsingError doesn't seem to be an apt error for this case
        .ok_or(errors::ParsingError::UnknownError)
        .into_report()?;

    Ok(item_object.notification_request_item)
}

#[async_trait::async_trait]
impl api::IncomingWebhook for Adyen {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let notif_item = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let base64_signature = notif_item.additional_data.hmac_signature;
        Ok(base64_signature.as_bytes().to_vec())
    }

    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _secret: &[u8],
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let message = format!(
            "{}:{}:{}:{}:{}:{}:{}:{}",
            notif.psp_reference,
            notif.original_reference.unwrap_or_default(),
            notif.merchant_account_code,
            notif.merchant_reference,
            notif.amount.value,
            notif.amount.currency,
            notif.event_code,
            notif.success
        );

        Ok(message.into_bytes())
    }

    async fn verify_webhook_source(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_account: &domain::MerchantAccount,
        merchant_connector_account: domain::MerchantConnectorAccount,
        connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let signature = self
            .get_webhook_source_verification_signature(request)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_account,
                connector_label,
                merchant_connector_account,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        let message = self
            .get_webhook_source_verification_message(
                request,
                &merchant_account.merchant_id,
                &connector_webhook_secrets.secret,
            )
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let raw_key = hex::decode(connector_webhook_secrets.secret)
            .into_report()
            .change_context(errors::ConnectorError::WebhookVerificationSecretInvalid)?;

        let signing_key = hmac::Key::new(hmac::HMAC_SHA256, &raw_key);
        let signed_messaged = hmac::sign(&signing_key, &message);
        let payload_sign = consts::BASE64_ENGINE.encode(signed_messaged.as_ref());
        Ok(payload_sign.as_bytes().eq(&signature))
    }

    fn get_webhook_object_reference_id(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookReferenceIdNotFound)?;
        // for capture_event, original_reference field will have the authorized payment's PSP reference
        if adyen::is_capture_event(&notif.event_code) {
            return Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    notif
                        .original_reference
                        .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                ),
            ));
        }
        if adyen::is_transaction_event(&notif.event_code) {
            return Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::PaymentAttemptId(notif.merchant_reference),
            ));
        }
        if adyen::is_refund_event(&notif.event_code) {
            return Ok(api_models::webhooks::ObjectReferenceId::RefundId(
                api_models::webhooks::RefundIdType::ConnectorRefundId(notif.psp_reference),
            ));
        }
        if adyen::is_chargeback_event(&notif.event_code) {
            return Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
                api_models::payments::PaymentIdType::ConnectorTransactionId(
                    notif
                        .original_reference
                        .ok_or(errors::ConnectorError::WebhookReferenceIdNotFound)?,
                ),
            ));
        }
        Err(errors::ConnectorError::WebhookReferenceIdNotFound).into_report()
    }

    fn get_webhook_event_type(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<IncomingWebhookEvent, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;
        Ok(IncomingWebhookEvent::foreign_from((
            notif.event_code,
            notif.additional_data.dispute_status,
        )))
    }

    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<serde_json::Value, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        let response: adyen::Response = notif.into();

        let res_json = serde_json::to_value(response)
            .into_report()
            .change_context(errors::ConnectorError::WebhookResourceObjectNotFound)?;

        Ok(res_json)
    }

    fn get_webhook_api_response(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ConnectorError>
    {
        Ok(services::api::ApplicationResponse::TextPlain(
            "[accepted]".to_string(),
        ))
    }

    fn get_dispute_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::disputes::DisputePayload, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(api::disputes::DisputePayload {
            amount: notif.amount.value.to_string(),
            currency: notif.amount.currency,
            dispute_stage: api_models::enums::DisputeStage::from(notif.event_code.clone()),
            connector_dispute_id: notif.psp_reference,
            connector_reason: notif.reason,
            connector_reason_code: notif.additional_data.chargeback_reason_code,
            challenge_required_by: notif.additional_data.defense_period_ends_at,
            connector_status: notif.event_code.to_string(),
            created_at: notif.event_date,
            updated_at: notif.event_date,
        })
    }
}

pub fn check_for_payment_method_balance(
    req: &types::PaymentsAuthorizeRouterData,
) -> CustomResult<(), errors::ConnectorError> {
    match &req.request.payment_method_data {
        api_models::payments::PaymentMethodData::GiftCard(gift_card) => match gift_card.as_ref() {
            api_models::payments::GiftCardData::Givex(_) => {
                let payment_method_balance = req
                    .payment_method_balance
                    .as_ref()
                    .ok_or(errors::ConnectorError::RequestEncodingFailed)?;
                if payment_method_balance.currency != req.request.currency.to_string()
                    || payment_method_balance.amount < req.request.amount
                {
                    Err(errors::ConnectorError::InSufficientBalanceInPaymentMethod.into())
                } else {
                    Ok(())
                }
            }
            _ => Ok(()),
        },
        _ => Ok(()),
    }
}
