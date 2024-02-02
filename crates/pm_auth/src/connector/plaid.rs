pub mod transformers;

use std::fmt::Debug;

use common_utils::{
    ext_traits::BytesExt,
    request::{Method, Request, RequestBuilder, RequestContent},
};
use error_stack::ResultExt;
use masking::{Mask, Maskable};
use transformers as plaid;

use crate::{
    core::errors,
    types::{
        self as auth_types,
        api::{
            auth_service::{self, BankAccountCredentials, ExchangeToken, LinkToken},
            ConnectorCommon, ConnectorCommonExt, ConnectorIntegration,
        },
    },
};

#[derive(Debug, Clone)]
pub struct Plaid;

impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Plaid
where
    Self: ConnectorIntegration<Flow, Request, Response>,
{
        /// Builds a vector of headers for a payment authorization request based on the provided payment authentication router data and payment method authentication connectors.
    /// 
    /// # Arguments
    /// 
    /// * `req` - The payment authentication router data containing the flow, request, and response.
    /// * `_connectors` - The payment method authentication connectors.
    /// 
    /// # Returns
    /// 
    /// A `CustomResult` containing a vector of tuples, where each tuple consists of a header name and a maskable header value, or a `ConnectorError` if an error occurs.
    /// 
    fn build_headers(
        &self,
        req: &auth_types::PaymentAuthRouterData<Flow, Request, Response>,
        _connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let mut header = vec![(
            "Content-Type".to_string(),
            self.get_content_type().to_string().into(),
        )];

        let mut auth = self.get_auth_header(&req.connector_auth_type)?;
        header.append(&mut auth);
        Ok(header)
    }
}

impl ConnectorCommon for Plaid {
    /// Returns the unique identifier for the object.
    fn id(&self) -> &'static str {
        "plaid"
    }

    /// Returns the content type "application/json".
    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }
    /// Returns the base URL for the Plaid API, which is "https://sandbox.plaid.com".
    fn base_url<'a>(&self, _connectors: &'a auth_types::PaymentMethodAuthConnectors) -> &'a str {
        "https://sandbox.plaid.com"
    }

    /// Retrieves the authentication header for the given ConnectorAuthType. The authentication header
    /// consists of the client ID and secret, both masked for security purposes. Returns a vector of tuples
    /// containing the header keys and their corresponding masked values.
    fn get_auth_header(
        &self,
        auth_type: &auth_types::ConnectorAuthType,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = plaid::PlaidAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let client_id = auth.client_id.into_masked();
        let secret = auth.secret.into_masked();

        Ok(vec![
            ("PLAID-CLIENT-ID".to_string(), client_id),
            ("PLAID-SECRET".to_string(), secret),
        ])
    }

    /// Builds an error response from the given auth_types::Response. It parses the response into a plaid::PlaidErrorResponse, and then constructs an auth_types::ErrorResponse from the status code, error code, error message, and display message obtained from the parsed PlaidErrorResponse. Returns a CustomResult containing the constructed ErrorResponse or a ConnectorError in case of response deserialization failure.
    fn build_error_response(
        &self,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ErrorResponse, errors::ConnectorError> {
        let response: plaid::PlaidErrorResponse =
            res.response
                .parse_struct("PlaidErrorResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        Ok(auth_types::ErrorResponse {
            status_code: res.status_code,
            code: crate::consts::NO_ERROR_CODE.to_string(),
            message: response.error_message,
            reason: response.display_message,
        })
    }
}

impl auth_service::AuthService for Plaid {}
impl auth_service::AuthServiceLinkToken for Plaid {}

impl ConnectorIntegration<LinkToken, auth_types::LinkTokenRequest, auth_types::LinkTokenResponse>
    for Plaid
{
        /// Retrieves the headers for a given request and payment method authentication connectors.
    fn get_headers(
        &self,
        req: &auth_types::LinkTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

        /// Returns the content type of the resource.
    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

        /// Retrieves the URL for creating a link token based on the provided connectors and router data.
    fn get_url(
        &self,
        _req: &auth_types::LinkTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/link/token/create"
        ))
    }

        /// Retrieves the request body for a given LinkTokenRouterData. 
    ///
    /// # Arguments
    ///
    /// * `req` - A reference to a LinkTokenRouterData object
    ///
    /// # Returns
    ///
    /// A result containing the request content as JSON or a ConnectorError if an error occurs.
    ///
    /// # Errors
    ///
    /// Returns a ConnectorError if the conversion from LinkTokenRouterData to PlaidLinkTokenRequest fails.
    fn get_request_body(
        &self,
        req: &auth_types::LinkTokenRouterData,
    ) -> errors::CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = plaid::PlaidLinkTokenRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

        /// Builds a request using the given data and connectors. The method constructs a POST request using the provided `req` and `connectors` data, and returns it as a `Result` inside an `Option`. If successful, the `Request` is wrapped inside `Some`, otherwise `None` is returned along with a `ConnectorError`.
    fn build_request(
        &self,
        req: &auth_types::LinkTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&auth_types::PaymentAuthLinkTokenType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(auth_types::PaymentAuthLinkTokenType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(auth_types::PaymentAuthLinkTokenType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }

        /// Handles the response from the Plaid API and converts it into a CustomResult containing LinkTokenRouterData
    fn handle_response(
        &self,
        data: &auth_types::LinkTokenRouterData,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::LinkTokenRouterData, errors::ConnectorError> {
        let response: plaid::PlaidLinkTokenResponse = res
            .response
            .parse_struct("PlaidLinkTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        <auth_types::LinkTokenRouterData>::try_from(auth_types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
        /// This method takes a Response object and returns a CustomResult containing an ErrorResponse on success
    /// or a ConnectorError on failure. It delegates the construction of the ErrorResponse to the build_error_response method of the current object.
    fn get_error_response(
        &self,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl auth_service::AuthServiceExchangeToken for Plaid {}

impl
    ConnectorIntegration<
        ExchangeToken,
        auth_types::ExchangeTokenRequest,
        auth_types::ExchangeTokenResponse,
    > for Plaid
{
        /// This method takes in a request and payment method authentication connectors, and returns a result containing a vector of tuples where the first element is a String and the second element is a Maskable String. The method calls the build_headers method with the provided request and connectors to generate the headers.
    fn get_headers(
        &self,
        req: &auth_types::ExchangeTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

        /// Constructs a URL for exchanging a public token using the provided payment method auth connectors.
    fn get_url(
        &self,
        _req: &auth_types::ExchangeTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}{}",
            self.base_url(connectors),
            "/item/public_token/exchange"
        ))
    }

        /// Retrieves the request body from the given ExchangeTokenRouterData and converts it into a PlaidExchangeTokenRequest. 
    /// If successful, it returns the request content as a JSON object boxed within RequestContent. 
    fn get_request_body(
        &self,
        req: &auth_types::ExchangeTokenRouterData,
    ) -> errors::CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = plaid::PlaidExchangeTokenRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

        /// Builds a request for exchanging a token with the payment method authentication service.
    fn build_request(
        &self,
        req: &auth_types::ExchangeTokenRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&auth_types::PaymentAuthExchangeTokenType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(auth_types::PaymentAuthExchangeTokenType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(auth_types::PaymentAuthExchangeTokenType::get_request_body(
                    self, req,
                )?)
                .build(),
        ))
    }

        /// Handles the response from the Plaid API exchange token request. It parses the response into a `PlaidExchangeTokenResponse`, changes the context of any deserialization errors to a `ConnectorError`, and then tries to convert the response and original data into an `ExchangeTokenRouterData`.
    fn handle_response(
        &self,
        data: &auth_types::ExchangeTokenRouterData,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ExchangeTokenRouterData, errors::ConnectorError> {
        let response: plaid::PlaidExchangeTokenResponse = res
            .response
            .parse_struct("PlaidExchangeTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        <auth_types::ExchangeTokenRouterData>::try_from(auth_types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}

impl auth_service::AuthServiceBankAccountCredentials for Plaid {}

impl
    ConnectorIntegration<
        BankAccountCredentials,
        auth_types::BankAccountCredentialsRequest,
        auth_types::BankAccountCredentialsResponse,
    > for Plaid
{
        /// Retrieves the headers required for making a request to the bank details router. 
    ///
    /// # Arguments
    ///
    /// * `req` - The bank details router data
    /// * `connectors` - The payment method authentication connectors
    ///
    /// # Returns
    ///
    /// Returns a Result containing a vector of tuples, where the first element is a String key and the second element is a Maskable String value. If successful, it returns the headers, otherwise it returns a ConnectorError.
    ///
    fn get_headers(
        &self,
        req: &auth_types::BankDetailsRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

        /// This method takes in a BankDetailsRouterData reference and a PaymentMethodAuthConnectors reference, and returns a CustomResult containing a String. It formats the base URL using the connectors and appends "/auth/get" to it.
    fn get_url(
        &self,
        _req: &auth_types::BankDetailsRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}{}", self.base_url(connectors), "/auth/get"))
    }

        /// Retrieves the request body by converting the provided BankDetailsRouterData into a PlaidBankAccountCredentialsRequest and wrapping it in a RequestContent::Json. Returns a CustomResult containing the request content if successful, or a ConnectorError if an error occurs.
    fn get_request_body(
        &self,
        req: &auth_types::BankDetailsRouterData,
    ) -> errors::CustomResult<RequestContent, errors::ConnectorError> {
        let req_obj = plaid::PlaidBankAccountCredentialsRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(req_obj)))
    }

        /// Constructs a request to retrieve bank account details using the provided data and connectors.
    fn build_request(
        &self,
        req: &auth_types::BankDetailsRouterData,
        connectors: &auth_types::PaymentMethodAuthConnectors,
    ) -> errors::CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&auth_types::PaymentAuthBankAccountDetailsType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(auth_types::PaymentAuthBankAccountDetailsType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(
                    auth_types::PaymentAuthBankAccountDetailsType::get_request_body(self, req)?,
                )
                .build(),
        ))
    }

        /// Handles the response from the Plaid API by parsing the response data, creating a new BankDetailsRouterData object, and returning a CustomResult
    fn handle_response(
        &self,
        data: &auth_types::BankDetailsRouterData,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::BankDetailsRouterData, errors::ConnectorError> {
        let response: plaid::PlaidBankAccountCredentialsResponse = res
            .response
            .parse_struct("PlaidBankAccountCredentialsResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        <auth_types::BankDetailsRouterData>::try_from(auth_types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }
    fn get_error_response(
        &self,
        res: auth_types::Response,
    ) -> errors::CustomResult<auth_types::ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}
