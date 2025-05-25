# Connector Integration Guide

This document provides a comprehensive guide for integrating new connectors into the Hyperswitch platform. It outlines the common practices, potential pitfalls, and best practices to ensure a smooth and efficient integration process.

## 1. Understanding the Connector Architecture

The Hyperswitch connector architecture is designed to be modular and extensible. Each connector is implemented as a separate Rust crate that adheres to a specific interface. This allows for easy integration of new connectors without affecting the core platform.

### 1.1 Key Components

*   **Connector Integration Trait:** The `ConnectorIntegration` trait defines the core interface that all connectors must implement. This trait includes functions for handling various payment flows, such as authorization, capture, and refund.
*   **Router Data:** The `RouterData` struct encapsulates all the information required for a specific payment flow, including the request data, connector configuration, and merchant information.
*   **Transformers:** Transformers are responsible for mapping the Hyperswitch data structures to the connector-specific data structures and vice versa.
*   **Types:** The `types` module defines the data structures used for requests and responses to the connector.
*   **Utils:** The `utils` module provides utility functions that are used throughout the connector implementation.

### 1.2 Connector Template

The `connector-template` directory provides a basic template for creating new connectors. This template includes the basic file structure and code snippets that can be used as a starting point for the integration process.

## 2. Implementing the Connector Integration Trait

The `ConnectorIntegration` trait defines the core interface that all connectors must implement. This trait includes functions for handling various payment flows, such as authorization, capture, and refund.

### 2.1 Required Functions

The following functions must be implemented for each payment flow:

*   `get_headers`: This function is responsible for constructing the headers that will be sent with the request to the connector.
*   `get_url`: This function is responsible for constructing the URL that will be used to send the request to the connector.
*   `get_request_body`: This function is responsible for constructing the request body that will be sent to the connector.
*   `handle_response`: This function is responsible for handling the response from the connector and converting it into a standard format.
*   `get_error_response`: This function is responsible for handling errors from the connector and converting them into a standard `ErrorResponse` format.

### 2.2 Code Example

```rust
impl
    ConnectorIntegration<
        Authorize,
        PaymentsAuthorizeData,
        PaymentsResponseData,
    > for {{project-name | downcase | pascal_case}} {
    fn get_headers(&self, req: &PaymentsAuthorizeRouterData, connectors: &Connectors,) -> CustomResult<Vec<(String, masking::Maskable<String>)>,errors::ConnectorError> {
        self.build_headers(req, connectors)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(&self, _req: &PaymentsAuthorizeRouterData, _connectors: &Connectors,) -> CustomResult<String,errors::ConnectorError> {
        Err(errors::ConnectorError::NotImplemented("get_url method".to_string()).into())
    }

    fn get_request_body(&self, req: &PaymentsAuthorizeRouterData, _connectors: &Connectors,) -> CustomResult<RequestContent, errors::ConnectorError> {
        let amount = utils::convert_amount(
            self.amount_converter,
            req.request.minor_amount,
            req.request.currency,
        )?;

        let connector_router_data =
            {{project-name | downcase}}::{{project-name | downcase | pascal_case}}RouterData::from((
                amount,
                req,
            ));
        let connector_req = {{project-name | downcase}}::{{project-name | downcase | pascal_case}}PaymentsRequest::try_from(&connector_router_data)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn build_request(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::PaymentsAuthorizeType::get_url(
                    self, req, connectors,
                )?)
                .attach_default_headers()
                .headers(types::PaymentsAuthorizeType::get_headers(
                    self, req, connectors,
                )?)
                .set_body(types::PaymentsAuthorizeType::get_request_body(self, req, connectors)?)
                .build(),
        ))
    }

    fn handle_response(
        &self,
        data: &PaymentsAuthorizeRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<PaymentsAuthorizeRouterData,errors::ConnectorError> {
        let response: {{project-name | downcase}}::{{project-name | downcase | pascal_case}}PaymentsResponse = res.response.parse_struct("{{project-name | downcase | pascal_case}} PaymentsAuthorizeResponse").change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
    }

    fn get_error_response(&self, res: Response, event_builder: Option<&mut ConnectorEvent>) -> CustomResult<ErrorResponse,errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}
```

## 3. Implementing the ConnectorCommon Trait

The `ConnectorCommon` trait defines common functions that are used by all connectors. This trait includes functions for getting the connector ID, currency unit, content type, base URL, and authentication header.

### 3.1 Required Functions

The following functions must be implemented for each connector:

*   `id`: This function returns the unique identifier for the connector.
*   `get_currency_unit`: This function returns the currency unit that the connector uses.
*   `common_get_content_type`: This function returns the content type that the connector uses.
*   `base_url`: This function returns the base URL for the connector API.
*   `get_auth_header`: This function returns the authentication header that will be used to authenticate with the connector API.
*   `build_error_response`: This function builds a standard error response from the connector's error response.

### 3.2 Code Example

```rust
impl ConnectorCommon for {{project-name | downcase | pascal_case}} {
    fn id(&self) -> &'static str {
        "{{project-name | downcase}}"
    }

    fn get_currency_unit(&self) -> api::CurrencyUnit {
        todo!()
    //    TODO! Check connector documentation, on which unit they are processing the currency.
    //    If the connector accepts amount in lower unit ( i.e cents for USD) then return api::CurrencyUnit::Minor,
    //    if connector accepts amount in base unit (i.e dollars for USD) then return api::CurrencyUnit::Base
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.{{project-name}}.base_url.as_ref()
    }

    fn get_auth_header(&self, auth_type:&ConnectorAuthType)-> CustomResult<Vec<(String,masking::Maskable<String>)>,errors::ConnectorError> {
        let auth =  {{project-name | downcase}}::{{project-name | downcase | pascal_case}}AuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        Ok(vec![(headers::AUTHORIZATION.to_string(), auth.api_key.expose().into_masked())])
    }

    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: {{project-name | downcase}}::{{project-name | downcase | pascal_case}}ErrorResponse = res
            .response
            .parse_struct("{{project-name | downcase | pascal_case}}ErrorResponse")
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
            network_decline_code: None,
            network_advice_code: None,
            network_error_message: None,
        })
    }
}
```

## 4. Deep Dive into Transformer Files

Transformers are responsible for mapping the Hyperswitch data structures to the connector-specific data structures and vice versa. This is a crucial step in the integration process, as it ensures that the data is in the correct format for the connector API.

### 4.1 Request Transformers

Request transformers are responsible for mapping the Hyperswitch request data to the connector-specific request data. This typically involves creating a new struct that implements the `TryFrom` trait for the Hyperswitch request data.

### 4.2 Response Transformers

Response transformers are responsible for mapping the connector-specific response data to the Hyperswitch response data. This typically involves creating a new struct that implements the `TryFrom` trait for the connector response data.

## 5. Handling Errors

Error handling is a crucial aspect of connector integration. It is important to handle errors gracefully and provide informative error messages to the user.

### 5.1 Error Responses

The `get_error_response` function is responsible for handling errors from the connector and converting them into a standard `ErrorResponse` format. This function should parse the connector-specific error response and extract the relevant information, such as the error code, message, and reason.

### 5.2 Error Codes

It is important to use consistent error codes across all connectors. The following error codes are recommended:

*   `200`: Success
*   `400`: Bad Request
*   `401`: Unauthorized
*   `403`: Forbidden
*   `404`: Not Found
*   `500`: Internal Server Error

## 6. Testing

Testing is a crucial part of the connector integration process. It is important to thoroughly test the connector to ensure that it is working correctly and that it is handling all possible scenarios.

### 6.1 Unit Tests

Unit tests should be written for all core functions in the connector implementation. This includes the `get_headers`, `get_url`, `get_request_body`, `handle_response`, and `get_error_response` functions.

### 6.2 Integration Tests

Integration tests should be written to test the end-to-end flow of the connector. This includes testing the authorization, capture, and refund flows.

## 7. Security

Security is a top priority for Hyperswitch. All connectors must be implemented with security in mind.

### 7.1 Data Masking

Sensitive data, such as API keys and card numbers, must be masked using the `masking` crate. This prevents sensitive data from being logged or exposed in error messages.

### 7.2 Input Validation

All input data must be validated to prevent injection attacks and other security vulnerabilities.

## 8. Best Practices

*   Follow the existing code style and conventions.
*   Write clear and concise code.
*   Add comments to explain complex logic.
*   Write unit tests for all core functions.
*   Test the connector thoroughly before submitting it for review.
*   Use descriptive variable names.
*   Handle errors gracefully.
*   Mask sensitive data.
*   Validate input data.

## 9. Common Pitfalls

*   Incorrectly mapping data structures.
*   Not handling errors gracefully.
*   Exposing sensitive data.
*   Not validating input data.
*   Not writing unit tests.
*   Not testing the connector thoroughly.

By following this guide, you can ensure that your connector integration is smooth, efficient, and secure.
