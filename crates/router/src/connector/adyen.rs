pub mod transformers;

use std::fmt::Debug;

use api_models::webhooks::IncomingWebhookEvent;
use base64::Engine;
use common_utils::request::RequestContent;
use diesel_models::{enums as storage_enums, enums};
use error_stack::{IntoReport, ResultExt};
use ring::hmac;
use router_env::{instrument, tracing};

use self::transformers as adyen;
use super::utils as connector_utils;
use crate::{
    configs::settings,
    consts,
    core::errors::{self, CustomResult},
    headers, logger,
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
    utils::{crypto, ByteSliceExt, BytesExt, OptionExt},
};

#[derive(Debug, Clone)]
pub struct Adyen;

impl ConnectorCommon for Adyen {
        /// This method returns the unique identifier "adyen".
    fn id(&self) -> &'static str {
        "adyen"
    }

        /// This method returns the currency unit of the API. In this case, it always returns the minor currency unit.
    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor
    }

        /// Retrieves the authentication header for a given ConnectorAuthType. This method
    /// attempts to convert the provided auth_type into an AdyenAuthType and obtain the
    /// API key from it. The API key is then returned as part of a vector containing a
    /// tuple with the X_API_KEY header and the masked API key value.
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
        /// Returns the base URL for the Adyen connector.
    fn base_url<'a>(&self, connectors: &'a settings::Connectors) -> &'a str {
        connectors.adyen.base_url.as_ref()
    }

        /// Builds an error response based on the provided response and returns a CustomResult containing the error response or a ConnectorError if deserialization of the response fails.
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
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl ConnectorValidation for Adyen {
        /// Validates the given capture method and returns a result indicating whether it is valid or not.
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
        /// Validates the reference ID for payments synchronization. 
    /// 
    /// If the provided data contains an encoded data, the method returns Ok(()), indicating that the reference ID is valid. 
    /// If the encoded data is missing, the method returns an error of type ConnectorError with a message indicating the missing field.
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
        /// Returns true if the source verification is mandatory for the webhook, otherwise returns false.
    fn is_webhook_source_verification_mandatory(&self) -> bool {
        true
    }
}

impl api::Payment for Adyen {}
impl api::PaymentAuthorize for Adyen {}
impl api::PaymentSync for Adyen {}
impl api::PaymentVoid for Adyen {}
impl api::PaymentCapture for Adyen {}
impl api::MandateSetup for Adyen {}
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
        api::SetupMandate,
        types::SetupMandateRequestData,
        types::PaymentsResponseData,
    > for Adyen
{
        /// Retrieves the headers required for setting up a mandate router, based on the provided request data and connectors. 
    fn get_headers(
        &self,
        req: &types::SetupMandateRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::SetupMandateType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

        /// Returns the URL for making payments using the setup mandate router data and connectors provided.
    fn get_url(
        &self,
        _req: &types::SetupMandateRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "v68/payments"))
    }
        /// Retrieves the request body for setting up a mandate router, based on the provided setup mandate router data
    /// and connectors settings. It constructs an authorize request, converts it to AdyenRouterData, then to
    /// AdyenPaymentRequest, and returns the request content as a JSON object wrapped in a CustomResult.
    fn get_request_body(
        &self,
        req: &types::SetupMandateRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
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

        Ok(RequestContent::Json(Box::new(connector_req)))
    }
        /// Builds a request for setting up a mandate by using the provided SetupMandateRouterData and Connectors. 
    fn build_request(
        &self,
        req: &types::SetupMandateRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::SetupMandateType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::SetupMandateType::get_headers(self, req, connectors)?)
                .set_body(types::SetupMandateType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }
        /// Handles the response from the setup mandate router by parsing the response into an AdyenPaymentResponse, and then creating a RouterData object using the parsed response, the given data, and the HTTP status code. The method returns a CustomResult containing the RouterData or an error of type ConnectorError.
    fn handle_response(
        &self,
        data: &types::SetupMandateRouterData,
        res: types::Response,
    ) -> CustomResult<
        types::RouterData<
            api::SetupMandate,
            types::SetupMandateRequestData,
            types::PaymentsResponseData,
        >,
        errors::ConnectorError,
    >
    where
        api::SetupMandate: Clone,
        types::SetupMandateRequestData: Clone,
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
        /// This method takes a types::Response and converts it into a CustomResult containing a types::ErrorResponse or an errors::ConnectorError. It first parses the response into an adyen::ErrorResponse struct, handling any deserialization errors with the appropriate error context. It then constructs a types::ErrorResponse from the status_code and error details of the parsed response, setting reason, attempt_status, and connector_transaction_id to None.
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
            attempt_status: None,
            connector_transaction_id: None,
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
        /// Retrieves the headers required for making a payment capture request. 
    /// The method takes the PaymentsCaptureRouterData and Connectors as input and returns a vector of tuples containing headers and masked values.
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

        /// Retrieves the URL for capturing a payment, based on the provided PaymentsCaptureRouterData and Connectors.
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
        /// Retrieves the request body for capturing payments using the provided payment capture router data and connectors settings. 
    ///
    /// # Arguments
    ///
    /// * `req` - The payment capture router data containing information about the payment capture request
    /// * `_connectors` - The connectors settings used for the payment capture request
    ///
    /// # Returns
    ///
    /// Returns a `CustomResult` containing the request content for capturing payments, or a `ConnectorError` if an error occurs.
    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount_to_capture,
            req,
        ))?;
        let connector_req = adyen::AdyenCaptureRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
        /// Builds a request for capturing payments based on the provided data and connectors.
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
                .set_body(types::PaymentsCaptureType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }
        /// Handles the response from a payments capture router by parsing the response into an AdyenCaptureResponse
    /// and creating a new RouterData object with the parsed response, original data, and HTTP status code.
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
        /// Retrieves the error response from a given HTTP response. It deserializes the response body into an adyen::ErrorResponse struct and constructs a types::ErrorResponse from the fields of the HTTP response and the deserialized adyen::ErrorResponse. If deserialization fails, it returns a ConnectorError.
    
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
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

/// Payment Sync can be useful only incase of Redirect flow.
/// For payments which doesn't involve redrection we have to rely on webhooks.
impl
    services::ConnectorIntegration<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>
    for Adyen
{
        /// Retrieves the headers required for making a request to the specified API endpoint. This method takes the request data, including the API type and response data, as well as the connectors settings. It then constructs the necessary headers, including the content type and authentication headers, and returns them as a vector of tuples containing the header name and value. If an error occurs during the construction of the headers, a `ConnectorError` is returned.
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

        /// Retrieves the request body from the provided RouterData and Connectors, and processes the encoded data to create a connector request for Adyen redirection or refusal. Returns the connector request as a JSON RequestContent wrapped in a CustomResult.
    
    fn get_request_body(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let encoded_data = req
            .request
            .encoded_data
            .clone()
            .get_required_value("encoded_data")
            .change_context(errors::ConnectorError::RequestEncodingFailed)?;
        let adyen_redirection_type = serde_urlencoded::from_str::<
            transformers::AdyenRedirectRequestTypes,
        >(encoded_data.as_str())
        .into_report()
        .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let connector_req = match adyen_redirection_type {
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
            adyen::AdyenRedirectRequestTypes::AdyenThreeDS(req) => adyen::AdyenRedirectRequest {
                details: adyen::AdyenRedirectRequestTypes::AdyenThreeDS(adyen::AdyenThreeDS {
                    three_ds_result: req.three_ds_result,
                    type_of_redirection_result: None,
                    result_code: None,
                }),
            },
            adyen::AdyenRedirectRequestTypes::AdyenRefusal(req) => adyen::AdyenRedirectRequest {
                details: adyen::AdyenRedirectRequestTypes::AdyenRefusal(adyen::AdyenRefusal {
                    payload: req.payload,
                    type_of_redirection_result: None,
                    result_code: None,
                }),
            },
        };

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

        /// This method takes in a reference to self, a reference to types::RouterData, a reference to settings::Connectors, and returns a CustomResult containing a String or a ConnectorError. It constructs a URL by combining the base URL from the provided connectors and a specific path for payments details, and returns the constructed URL as a String.
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

        /// Builds a request based on the provided RouterData and Connectors. The method checks if the PSync flow is supported and creates a request accordingly, taking into account the encoded data and the redirection flow. If the encoded data contains the redirect URL, a POST request is built with the appropriate URL, headers, and request body. Otherwise, the method returns None.
    fn build_request(
        &self,
        req: &types::RouterData<api::PSync, types::PaymentsSyncData, types::PaymentsResponseData>,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        // Adyen doesn't support PSync flow. We use PSync flow to fetch payment details,
        // specifically the redirect URL that takes the user to their Payment page. In non-redirection flows,
        // we rely on webhooks to obtain the payment status since there is no encoded data available.
        // encoded_data only includes the redirect URL and is only relevant in redirection flows.
        if req
            .request
            .encoded_data
            .clone()
            .get_required_value("encoded_data")
            .is_ok()
        {
            Ok(Some(
                services::RequestBuilder::new()
                    .method(services::Method::Post)
                    .url(&types::PaymentsSyncType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(types::PaymentsSyncType::get_headers(self, req, connectors)?)
                    .set_body(types::PaymentsSyncType::get_request_body(
                        self, req, connectors,
                    )?)
                    .build(),
            ))
        } else {
            Ok(None)
        }
    }

    /// Handles the response from the payment sync API by parsing the response, determining the sync type, and creating a new PaymentsSyncRouterData instance. 
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
            attempt_status: None,
            connector_transaction_id: None,
        })
    }

        /// Returns the capture synchronization method for multiple captures.
    fn get_multiple_capture_sync_method(
        &self,
    ) -> CustomResult<services::CaptureSyncMethod, errors::ConnectorError> {
        Ok(services::CaptureSyncMethod::Individual)
    }
}

impl
    services::ConnectorIntegration<
        api::Authorize,
        types::PaymentsAuthorizeData,
        types::PaymentsResponseData,
    > for Adyen
{
        /// This method returns a vector of headers required for authorizing payments. It takes the
    /// request data, connectors settings, and returns a result containing a vector of tuples
    /// representing headers. The method also enforces the constraint that the implementing type
    /// must be a ConnectorIntegration for the Authorize API with specific data and response types.
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

        /// Constructs and returns a URL for payments authorization using the provided connector settings.
    ///
    /// # Arguments
    ///
    /// * `self` - The reference to the current instance of the struct.
    /// * `_req` - The reference to the payments authorization router data.
    /// * `connectors` - The reference to the connector settings.
    ///
    /// # Returns
    ///
    /// A Result containing the constructed URL for payments authorization, or a ConnectorError if an error occurs.
    ///
    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "v68/payments"))
    }
        /// Retrieves the request body for authorizing a payment through the Adyen payment gateway. 
    /// 
    /// # Arguments
    /// - `req`: The payment authorization router data containing necessary information for the request
    /// - `_connectors`: The connectors settings used for the request
    /// 
    /// # Returns
    /// Returns a `CustomResult` containing the request content for authorizing a payment or an error if the request fails.
    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = adyen::AdyenPaymentRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

        /// Builds a request for authorizing payments using the provided router data and connectors.
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

        /// Handles the response from the payment authorization router by parsing the response into an AdyenPaymentResponse,
    /// creating a new ResponseRouterData, and attempting to convert it into RouterData. If successful, it returns the
    /// RouterData, otherwise it returns a ConnectorError.
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
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl api::PaymentsPreProcessing for Adyen {}

impl
    services::ConnectorIntegration<
        api::PreProcessing,
        types::PaymentsPreProcessingData,
        types::PaymentsResponseData,
    > for Adyen
{
        /// Retrieves the headers required for making a payments pre-processing request. It constructs the necessary headers based on the provided request data and connectors settings. It returns a vector of key-value pairs representing the headers, or an error of type `ConnectorError` if there is an issue with retrieving the headers or constructing the API key.
    fn get_headers(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            headers::CONTENT_TYPE.to_string(),
            types::PaymentsPreProcessingType::get_content_type(self)
                .to_string()
                .into(),
        )];
        let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut api_key);
        Ok(header)
    }

        /// Returns the URL for retrieving the balance of payment methods using the base URL from the provided connectors settings.
    fn get_url(
        &self,
        _req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}v69/paymentMethods/balance",
            self.base_url(connectors)
        ))
    }

        /// Retrieves the request body for the payment pre-processing router data. The method takes the payment pre-processing router data, along with the connectors settings, and returns the result as a custom result containing the request content or a connector error.
    fn get_request_body(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenBalanceRequest::try_from(req)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

        /// Builds a request for the PaymentsPreProcessingRouterData using the provided connectors.
    /// Returns a Result containing an Option of the constructed Request, or a ConnectorError if an error occurs.
    fn build_request(
        &self,
        req: &types::PaymentsPreProcessingRouterData,
        connectors: &settings::Connectors,
    ) -> CustomResult<Option<services::Request>, errors::ConnectorError> {
        Ok(Some(
            services::RequestBuilder::new()
                .method(services::Method::Post)
                .url(&types::PaymentsPreProcessingType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsPreProcessingType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsPreProcessingType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

        /// Handles the response from the payment pre-processing router by parsing the response,
    /// checking if the balance is sufficient, and either returning an error response or
    /// converting the response and data into a RouterData.
    fn handle_response(
        &self,
        data: &types::PaymentsPreProcessingRouterData,
        res: types::Response,
    ) -> CustomResult<types::PaymentsPreProcessingRouterData, errors::ConnectorError> {
        let response: adyen::AdyenBalanceResponse = res
            .response
            .parse_struct("AdyenBalanceResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        let currency = match data.request.currency {
            Some(currency) => currency,
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "currency",
            })?,
        };
        let amount = match data.request.amount {
            Some(amount) => amount,
            None => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "amount",
            })?,
        };

        if response.balance.currency != currency || response.balance.value < amount {
            Ok(types::RouterData {
                response: Err(types::ErrorResponse {
                    code: consts::NO_ERROR_CODE.to_string(),
                    message: consts::NO_ERROR_MESSAGE.to_string(),
                    reason: Some(consts::LOW_BALANCE_ERROR_MESSAGE.to_string()),
                    status_code: res.status_code,
                    attempt_status: Some(enums::AttemptStatus::Failure),
                    connector_transaction_id: None,
                }),
                ..data.clone()
            })
        } else {
            types::RouterData::try_from(types::ResponseRouterData {
                response,
                data: data.clone(),
                http_code: res.status_code,
            })
            .change_context(errors::ConnectorError::ResponseHandlingFailed)
        }
    }

        /// This method takes a `types::Response` and returns a `CustomResult` containing either a `types::ErrorResponse` or an `errors::ConnectorError`. It calls the `build_error_response` method on the current instance to construct the error response.
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
        /// Retrieves the headers required for making a request to the PaymentsCancelRouterData endpoint. This method constructs the required headers, including the content type and authentication headers, and returns them as a vector of key-value pairs.
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

        /// Returns the URL for canceling a payment transaction based on the provided PaymentsCancelRouterData and Connectors.
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

        /// Retrieves the request body for cancelling a payment, based on the provided PaymentsCancelRouterData and Connectors settings.
    ///
    /// # Arguments
    ///
    /// * `req` - The PaymentsCancelRouterData containing the necessary data for cancelling a payment.
    /// * `_connectors` - The Connectors settings used to configure the request.
    ///
    /// # Returns
    ///
    /// The result of retrieving the request body, which is either the JSON content of the AdyenCancelRequest or an error indicating a failure to retrieve the request body.
    fn get_request_body(
        &self,
        req: &types::PaymentsCancelRouterData,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenCancelRequest::try_from(req)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
        }
        /// Builds a request for cancelling payments using the provided payment cancellation router data and connectors.
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
                .set_body(types::PaymentsVoidType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

        /// Handles the response from the Adyen payment cancelation API and returns a result containing the updated payment cancelation router data or an error.
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
            attempt_status: None,
            connector_transaction_id: None,
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
        /// This method takes in a PayoutsRouterData object and a Connectors object, and returns a result containing a string. It formats the URL for declining a third party payout using the secondary base URL of the Adyen connector from the Connectors object.
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

        /// Retrieves the headers required for performing a payout cancellation request. It constructs the headers with the necessary content type and authentication information based on the provided PayoutsRouterData and Connectors.
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

        /// Retrieves the request body for cancelling a payout through the Adyen connector. 
    /// 
    /// # Arguments
    /// * `req` - The payout router data containing information about the payout cancellation.
    /// * `_connectors` - The connectors settings used for the request.
    /// 
    /// # Returns
    /// A `CustomResult` containing the request content for cancelling the payout, or an error of type `ConnectorError`.
    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoCancel>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = adyen::AdyenPayoutCancelRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

        /// Builds a request to cancel a payout using the provided PayoutsRouterData and Connectors.
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
            .set_body(types::PayoutCancelType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
        /// Handles the response from the Adyen payout API, parsing the response and creating a new RouterData object with the parsed response, original data, and HTTP status code.
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
        /// This method constructs and returns the URL for making a payout request using the provided connectors and request data.
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

        /// Retrieves the headers required for making a payout request. It constructs the headers by setting the content type and adding the API key obtained from the connector authentication type. Returns a vector of tuples containing the header names and their corresponding maskable values.
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

        /// Retrieves the request body for a payout creation request to the Adyen connector. 
    /// 
    /// # Arguments
    /// 
    /// * `req` - The payouts router data for the request.
    /// * `_connectors` - The connectors settings.
    /// 
    /// # Returns
    /// 
    /// * `CustomResult<RequestContent, errors::ConnectorError>` - A result containing the request content or a connector error.
    /// 
    /// # Errors
    /// 
    /// Returns a `ConnectorError` if there is an error creating the Adyen router data or payout create request.
    /// 
    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoCreate>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.destination_currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = adyen::AdyenPayoutCreateRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

        /// Builds a request for creating a payout using the provided PayoutsRouterData and Connectors.
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
            .set_body(types::PayoutCreateType::get_request_body(
                self, req, connectors,
            )?)
            .build();
    
        Ok(Some(request))
    }

    #[instrument(skip_all)]
        /// Handles the response from Adyen Payout API by parsing the response into AdyenPayoutResponse
    /// struct and creating a new RouterData instance with the parsed response, original data, and 
    /// HTTP status code. Returns a CustomResult containing the new PayoutsRouterData or a 
    /// ConnectorError if response deserialization fails.
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
        /// Returns a URL for payments by formatting the base URL with the given connectors and appending '/v68/payments' to it.
    fn get_url(
        &self,
        _req: &types::PayoutsRouterData<api::PoEligibility>,
        connectors: &settings::Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}v68/payments", self.base_url(connectors),))
    }

        /// This method retrieves the headers required for making a payout request. It takes the PayoutsRouterData, connectors settings, and returns a result containing a vector of header key-value pairs or a ConnectorError.
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

        /// Retrieves the request body for a payout transaction based on the provided PayoutsRouterData and Connectors. 
    /// 
    /// # Arguments
    /// 
    /// * `req` - The PayoutsRouterData containing the necessary information for the payout request.
    /// * `_connectors` - The Connectors used for the payout transaction.
    /// 
    /// # Returns
    /// 
    /// Returns a Result with the request body content or a ConnectorError if there is an error in the process.
    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoEligibility>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.destination_currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = adyen::AdyenPayoutEligibilityRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

        /// Builds a request for the PayoutsRouterData using the provided connectors, and returns the result as a CustomResult<Option<services::Request>, errors::ConnectorError>.
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
            .set_body(types::PayoutEligibilityType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
        /// Handles the response from the Adyen Payout API by parsing the response into an AdyenPayoutResponse struct, and then creating a ResponseRouterData from the parsed response, the original data, and the HTTP status code. Returns a CustomResult with the ResponseRouterData on success, or a ConnectorError on failure.
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
        /// Constructs and returns a URL based on the provided PayoutsRouterData and Connectors.
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

        /// This method retrieves headers required for performing a payout request. It constructs the necessary headers including content type and API key based on the payout type and authentication type. It returns a result containing a vector of tuples representing the headers and their values, or an error of type ConnectorError if there is a failure in obtaining the authentication type.
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

        /// Retrieves the request body for a PayoutsRouterData containing Adyen payout fulfillment data,
    /// using the specified connectors settings. It creates AdyenRouterData and AdyenPayoutFulfillRequest
    /// objects based on the provided PayoutsRouterData, and returns the request content as JSON.
    fn get_request_body(
        &self,
        req: &types::PayoutsRouterData<api::PoFulfill>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.destination_currency,
            req.request.amount,
            req,
        ))?;
        let connector_req = adyen::AdyenPayoutFulfillRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

        /// Constructs a request for fulfilling a payout using the given data and connectors. 
    /// Returns a custom result containing the constructed request, or an error if the request construction failed.
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
            .set_body(types::PayoutFulfillType::get_request_body(
                self, req, connectors,
            )?)
            .build();

        Ok(Some(request))
    }

    #[instrument(skip_all)]
        /// Handles the response received from the Adyen API for a payout request. It parses the response into an AdyenPayoutResponse struct, and then creates a new PayoutsRouterData object with the parsed response, the original data, and the HTTP status code from the response.
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
        /// Retrieves the headers needed for making a request to the refunds router API. 
    /// 
    /// # Arguments
    /// 
    /// * `req` - A reference to the refunds router data containing the request information.
    /// * `_connectors` - A reference to the settings for the connectors.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of tuples, where each tuple represents a header name-value pair. 
    /// 
    /// # Errors
    /// 
    /// Returns a `ConnectorError` if there is an error in retrieving the auth header. 
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

        /// This method takes the refunds router data and connectors settings as input and returns a custom result containing a string. It first extracts the connector transaction ID from the request data, then constructs a URL using the base URL from the connectors settings and the extracted connector transaction ID, and finally returns the constructed URL as a string inside a custom result.
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

        /// Retrieves the request body for a refunds router data and connectors, and returns a CustomResult containing the request content or a ConnectorError.
    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<api::Execute>,
        _connectors: &settings::Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_router_data = adyen::AdyenRouterData::try_from((
            &self.get_currency_unit(),
            req.request.currency,
            req.request.refund_amount,
            req,
        ))?;
        let connector_req = adyen::AdyenRefundRequest::try_from(&connector_router_data)?;

        Ok(RequestContent::Json(Box::new(connector_req)))
    }

        /// Builds a request for executing a refund using the provided data and connectors.
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
                .set_body(types::RefundExecuteType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }

    #[instrument(skip_all)]
        /// Handles the response from a refund request by parsing the response, creating a new RouterData object, and returning a CustomResult with the updated data or an error if the response parsing or handling fails.
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
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}

impl services::ConnectorIntegration<api::RSync, types::RefundsData, types::RefundsResponseData>
    for Adyen
{
}

/// Parses the given byte array as an AdyenIncomingWebhook object and extracts the first notification item. 
/// If successful, returns the AdyenNotificationRequestItemWH object inside a CustomResult. Otherwise, returns a ParsingError.
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
        /// Retrieves the verification algorithm used for verifying the source of a webhook request.
    /// 
    /// # Arguments
    /// 
    /// * `request` - The incoming webhook request details.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a boxed trait object implementing the `VerifySignature` and `Send` traits, representing the algorithm used for verifying the webhook source. Returns an error of type `ConnectorError` if the algorithm retrieval fails.
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }

        /// Retrieves the HMAC signature from the incoming webhook request details and returns it as a vector of bytes.
    fn get_webhook_source_verification_signature(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let notif_item = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let base64_signature = notif_item.additional_data.hmac_signature;
        Ok(base64_signature.as_bytes().to_vec())
    }

        /// Retrieves the verification message for the webhook source based on the incoming webhook request details, merchant ID, and connector webhook secrets. The method processes the webhook request body to extract notification details, constructs a message using the notification fields, and returns the message as a byte vector.
    fn get_webhook_source_verification_message(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &str,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
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

        /// Verifies the source of a webhook request by comparing the signature in the request
    /// with the signature generated using the webhook secrets associated with the
    /// merchant account and connector label. Returns a boolean indicating whether the
    /// verification was successful.
    async fn verify_webhook_source(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
        merchant_account: &domain::MerchantAccount,
        merchant_connector_account: domain::MerchantConnectorAccount,
        connector_label: &str,
    ) -> CustomResult<bool, errors::ConnectorError> {
        let connector_webhook_secrets = self
            .get_webhook_source_verification_merchant_secret(
                merchant_account,
                connector_label,
                merchant_connector_account,
            )
            .await
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let signature = self
            .get_webhook_source_verification_signature(request, &connector_webhook_secrets)
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)?;

        let message = self
            .get_webhook_source_verification_message(
                request,
                &merchant_account.merchant_id,
                &connector_webhook_secrets,
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

        /// Given an incoming webhook request, this method extracts the relevant information and returns an ObjectReferenceId
    /// based on the event code and other details in the request. It handles different types of webhook events such
    /// as capture, transaction, refund, and chargeback events, and constructs the corresponding ObjectReferenceId
    /// based on the event details.
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

        /// Retrieves the event type from the incoming webhook request details and returns a result containing the incoming webhook event or a ConnectorError if the event type is not found.
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

        /// Retrieves the webhook resource object from the incoming webhook request details and returns a custom result containing a boxed trait object that implements the ErasedMaskSerialize trait, or a ConnectorError if an error occurs. 
    fn get_webhook_resource_object(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookEventTypeNotFound)?;

        let response: adyen::Response = notif.into();

        Ok(Box::new(response))
    }

        /// This method takes an incoming webhook request details and returns a custom result containing the application response in JSON format. If successful, it returns a text plain response with the message "[accepted]". If there is an error, it returns a ConnectorError.
    fn get_webhook_api_response(
        &self,
        _request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<services::api::ApplicationResponse<serde_json::Value>, errors::ConnectorError>
    {
        Ok(services::api::ApplicationResponse::TextPlain(
            "[accepted]".to_string(),
        ))
    }

        /// Retrieves and maps the details of a dispute from the incoming webhook request to a `DisputePayload` object.
    fn get_dispute_details(
        &self,
        request: &api::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api::disputes::DisputePayload, errors::ConnectorError> {
        let notif = get_webhook_object_from_body(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        Ok(api::disputes::DisputePayload {
            amount: notif.amount.value.to_string(),
            currency: notif.amount.currency.to_string(),
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
