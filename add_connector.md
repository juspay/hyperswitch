# Guide to Integrating a Connector

## Table of Contents

1. [Introduction](#introduction)
2. [Prerequisites](#prerequisites)
3. [Understanding Connectors and Payment Methods](#understanding-connectors-and-payment-methods)
4. [Integration Steps](#integration-steps)
   - [Generate Template](#generate-template)
   - [Implement Request & Response Types](#implement-request--response-types)
   - [Implement transformers.rs](#implementing-transformersrs)
   - [Handle Response Mapping](#handle-response-mapping)
   - [Recommended Fields for Connector Request and Response](#recommended-fields-for-connector-request-and-response)
   - [Error Handling](#error-handling)
5. [Implementing the Traits](#implementing-the-traits)
   - [ConnectorCommon](#connectorcommon)
   - [ConnectorIntegration](#connectorintegration)
   - [ConnectorCommonExt](#connectorcommonext)
   - [Other Traits](#othertraits)
6. [Set the Currency Unit](#set-the-currency-unit)
7. [Connector utility functions](#connector-utility-functions)
8. [Connector configs for control center](#connector-configs-for-control-center)
9. [Update `ConnectorTypes.res` and `ConnectorUtils.res`](#update-connectortypesres-and-connectorutilsres)
10. [Add Connector Icon](#add-connector-icon)
11. [Test the Connector](#test-the-connector)
12. [Build Payment Request and Response from JSON Schema](#build-payment-request-and-response-from-json-schema)

## Introduction

This guide provides instructions on integrating a new connector with Router, from setting up the environment to implementing API interactions.

## Prerequisites

- Familiarity with the Connector API you’re integrating
- A locally set up and running Router repository
- API credentials for testing (sign up for sandbox/UAT credentials on the connector’s website).
- Rust nightly toolchain installed for code formatting:
  ```bash
  rustup toolchain install nightly
  ```

## Understanding Connectors and Payment Methods

A **Connector** processes payments (e.g., Stripe, Adyen) or manages fraud risk (e.g., Signifyd). A **Payment Method** is a specific way to transact (e.g., credit card, PayPal). See the [Hyperswitch Payment Matrix](https://hyperswitch.io/pm-list) for details.

## Integration Steps

Integrating a connector is mainly an API integration task. You'll define request and response types and implement required traits.

This tutorial covers card payments via [Billwerk](https://optimize-docs.billwerk.com/). Review the API reference and test APIs before starting.

Follow these steps to integrate a new connector.

### Generate Template

Run the following script to create the connector structure:

```bash
sh scripts/add_connector.sh <connector-name> <connector-base-url>
```

Example folder structure:

```
hyperswitch_connectors/src/connectors
├── billwerk
│   └── transformers.rs
└── billwerk.rs
crates/router/tests/connectors
└── billwerk.rs
```

**Note:** move the file `crates/hyperswitch_connectors/src/connectors/connector_name/test.rs` to `crates/router/tests/connectors/connector_name.rs`


Define API request/response types and conversions in `hyperswitch_connectors/src/connector/billwerk/transformers.rs`

Implement connector traits in `hyperswitch_connectors/src/connector/billwerk.rs`

Write basic payment flow tests in `crates/router/tests/connectors/billwerk.rs`

Boilerplate code with todo!() is provided—follow the guide and complete the necessary implementations.

### Implement Request & Response Types

Integrating a new connector involves transforming Router's core data into the connector's API format. Since the Connector module is stateless, Router handles data persistence.

#### Implementing transformers.rs

Design request/response structures based on the connector's API spec.

Define request format in `transformers.rs`:

```rust
#[derive(Debug, Serialize)]
pub struct BillwerkCustomerObject {
    handle: Option<id_type::CustomerId>,
    email: Option<Email>,
    address: Option<Secret<String>>,
    address2: Option<Secret<String>>,
    city: Option<String>,
    country: Option<common_enums::CountryAlpha2>,
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
}

#[derive(Debug, Serialize)]
pub struct BillwerkPaymentsRequest {
    handle: String,
    amount: MinorUnit,
    source: Secret<String>,
    currency: common_enums::Currency,
    customer: BillwerkCustomerObject,
    metadata: Option<SecretSerdeValue>,
    settle: bool,
}
```

Since Router is connector agnostic, only minimal data is sent to connector and optional fields may be ignored.

We transform our `PaymentsAuthorizeRouterData` into Billwerk's `PaymentsRequest` structure by employing the `try_from` function.

```rust
impl TryFrom<&BillwerkRouterData<&types::PaymentsAuthorizeRouterData>> for BillwerkPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &BillwerkRouterData<&types::PaymentsAuthorizeRouterData>,
    ) -> Result<Self, Self::Error> {
        if item.router_data.is_three_ds() {
            return Err(errors::ConnectorError::NotImplemented(
                "Three_ds payments through Billwerk".to_string(),
            )
            .into());
        };
        let source = match item.router_data.get_payment_method_token()? {
            PaymentMethodToken::Token(pm_token) => Ok(pm_token),
            _ => Err(errors::ConnectorError::MissingRequiredField {
                field_name: "payment_method_token",
            }),
        }?;
        Ok(Self {
            handle: item.router_data.connector_request_reference_id.clone(),
            amount: item.amount,
            source,
            currency: item.router_data.request.currency,
            customer: BillwerkCustomerObject {
                handle: item.router_data.customer_id.clone(),
                email: item.router_data.request.email.clone(),
                address: item.router_data.get_optional_billing_line1(),
                address2: item.router_data.get_optional_billing_line2(),
                city: item.router_data.get_optional_billing_city(),
                country: item.router_data.get_optional_billing_country(),
                first_name: item.router_data.get_optional_billing_first_name(),
                last_name: item.router_data.get_optional_billing_last_name(),
            },
            metadata: item.router_data.request.metadata.clone().map(Into::into),
            settle: item.router_data.request.is_auto_capture()?,
        })
    }
}
```

### Handle Response Mapping

When implementing the response type, the key enum to define for each connector is `PaymentStatus`. It represents the different status types returned by the connector, as specified in its API spec. Below is the definition for Billwerk.

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum BillwerkPaymentState {
    Created,
    Authorized,
    Pending,
    Settled,
    Failed,
    Cancelled,
}

impl From<BillwerkPaymentState> for enums::AttemptStatus {
    fn from(item: BillwerkPaymentState) -> Self {
        match item {
            BillwerkPaymentState::Created | BillwerkPaymentState::Pending => Self::Pending,
            BillwerkPaymentState::Authorized => Self::Authorized,
            BillwerkPaymentState::Settled => Self::Charged,
            BillwerkPaymentState::Failed => Self::Failure,
            BillwerkPaymentState::Cancelled => Self::Voided,
        }
    }
}
```

Here are common payment attempt statuses:

- **Charged:** Payment succeeded.
- **Pending:** Payment is processing.
- **Failure:** Payment failed.
- **Authorized:** Payment authorized; can be voided, captured, or partially captured.
- **AuthenticationPending:** Customer action required.
- **Voided:** Payment voided, funds returned to the customer.

**Note:** Default status should be `Pending`. Only explicit success or failure from the connector should mark the payment as `Charged` or `Failure`.

Define response format in `transformers.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BillwerkPaymentsResponse {
    state: BillwerkPaymentState,
    handle: String,
    error: Option<String>,
    error_state: Option<String>,
}
```

We transform our `ResponseRouterData` into `PaymentsResponseData` by employing the `try_from` function.

```rust
impl<F, T> TryFrom<ResponseRouterData<F, BillwerkPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<F, BillwerkPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let error_response = if item.response.error.is_some() || item.response.error_state.is_some()
        {
            Some(ErrorResponse {
                code: item
                    .response
                    .error_state
                    .clone()
                    .unwrap_or(NO_ERROR_CODE.to_string()),
                message: item
                    .response
                    .error_state
                    .unwrap_or(NO_ERROR_MESSAGE.to_string()),
                reason: item.response.error,
                status_code: item.http_code,
                attempt_status: None,
                connector_transaction_id: Some(item.response.handle.clone()),
            })
        } else {
            None
        };
        let payments_response = PaymentsResponseData::TransactionResponse {
            resource_id: ResponseId::ConnectorTransactionId(item.response.handle.clone()),
            redirection_data: Box::new(None),
            mandate_reference: Box::new(None),
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: Some(item.response.handle),
            incremental_authorization_allowed: None,
            charges: None,
        };
        Ok(Self {
            status: enums::AttemptStatus::from(item.response.state),
            response: error_response.map_or_else(|| Ok(payments_response), Err),
            ..item.data
        })
    }
}
```

### Recommended Fields for Connector Request and Response

- **connector_request_reference_id:** Merchant's reference ID in the payment request (e.g., `reference` in Checkout).

```rust
  reference: item.router_data.connector_request_reference_id.clone(),
```
- **connector_response_reference_id:** ID used for transaction identification in the connector dashboard, linked to merchant_reference or connector_transaction_id.

```rust
    connector_response_reference_id: item.response.reference.or(Some(item.response.id)),
```

- **resource_id:** The connector's connector_transaction_id is used as the resource_id. If unavailable, set to NoResponseId.

```rust
    resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
```

- **redirection_data:** For redirection flows (e.g., 3D Secure), assign the redirection link.

```rust
    let redirection_data = item.response.links.redirect.map(|href| {
        services::RedirectForm::from((href.redirection_url, services::Method::Get))
    });
```

### Error Handling

Define error responses:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct BillwerkErrorResponse {
    pub code: Option<i32>,
    pub error: String,
    pub message: Option<String>,
}
```

By following these steps, you can integrate a new connector efficiently while ensuring compatibility with Router's architecture.

## Implementing the Traits

The `mod.rs` file contains trait implementations using connector types in transformers. A struct with the connector name holds these implementations. Below are the mandatory traits:

### ConnectorCommon
Contains common description of the connector, like the base endpoint, content-type, error response handling, id, currency unit.

Within the `ConnectorCommon` trait, you'll find the following methods :

- `id` method corresponds directly to the connector name.

```rust
  fn id(&self) -> &'static str {
      "Billwerk"
  }
```

- `get_currency_unit` method anticipates you to [specify the accepted currency unit](#set-the-currency-unit) for the connector.

```rust
  fn get_currency_unit(&self) -> api::CurrencyUnit {
      api::CurrencyUnit::Minor
  }
```

- `common_get_content_type` method requires you to provide the accepted content type for the connector API.

```rust
  fn common_get_content_type(&self) -> &'static str {
      "application/json"
  }
```

- `get_auth_header` method accepts common HTTP Authorization headers that are accepted in all `ConnectorIntegration` flows.

```rust
    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        let auth = BillwerkAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        let encoded_api_key = BASE64_ENGINE.encode(format!("{}:", auth.api_key.peek()));
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Basic {encoded_api_key}").into_masked(),
        )])
    }
```

- `base_url` method is for fetching the base URL of connector's API. Base url needs to be consumed from configs.

```rust
    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.billwerk.base_url.as_ref()
    }
```

- `build_error_response` method is common error response handling for a connector if it is same in all cases

```rust
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: BillwerkErrorResponse = res
            .response
            .parse_struct("BillwerkErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response
                .code
                .map_or(NO_ERROR_CODE.to_string(), |code| code.to_string()),
            message: response.message.unwrap_or(NO_ERROR_MESSAGE.to_string()),
            reason: Some(response.error),
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
```

### ConnectorIntegration
For every api endpoint contains the url, using request transform and response transform and headers.
Within the `ConnectorIntegration` trait, you'll find the following methods implemented(below mentioned is example for authorized flow):

- `get_url` method defines endpoint for authorize flow, base url is consumed from `ConnectorCommon` trait.

```rust
    fn get_url(
        &self,
        _req: &TokenizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<String, errors::ConnectorError> {
        let base_url = connectors
            .billwerk
            .secondary_base_url
            .as_ref()
            .ok_or(errors::ConnectorError::FailedToObtainIntegrationUrl)?;
        Ok(format!("{base_url}v1/token"))
    }
```

- `get_headers` method accepts HTTP headers that are accepted for authorize flow. In this context, it is utilized from the `ConnectorCommonExt` trait, as the connector adheres to common headers across various flows.

```rust
    fn get_headers(
        &self,
        req: &TokenizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
        self.build_headers(req, connectors)
    }
```

- `get_request_body` method uses transformers to convert the Hyperswitch payment request to the connector's format. If successful, it returns the request as `RequestContent::Json`, supporting formats like JSON, form-urlencoded, XML, and raw bytes.

```rust
    fn get_request_body(
        &self,
        req: &TokenizationRouterData,
        _connectors: &Connectors,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let connector_req = BillwerkTokenRequest::try_from(req)?;
        Ok(RequestContent::Json(Box::new(connector_req)))
    }
```

- `build_request` method assembles the API request by providing the method, URL, headers, and request body as parameters.

```rust
    fn build_request(
        &self,
        req: &TokenizationRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Option<Request>, errors::ConnectorError> {
        Ok(Some(
            RequestBuilder::new()
                .method(Method::Post)
                .url(&types::TokenizationType::get_url(self, req, connectors)?)
                .attach_default_headers()
                .headers(types::TokenizationType::get_headers(self, req, connectors)?)
                .set_body(types::TokenizationType::get_request_body(
                    self, req, connectors,
                )?)
                .build(),
        ))
    }
```

- `handle_response` method calls transformers where connector response data is transformed into hyperswitch response.

```rust
    fn handle_response(
        &self,
        data: &TokenizationRouterData,
        event_builder: Option<&mut ConnectorEvent>,
        res: Response,
    ) -> CustomResult<TokenizationRouterData, errors::ConnectorError>
    where
        PaymentsResponseData: Clone,
    {
        let response: BillwerkTokenResponse = res
            .response
            .parse_struct("BillwerkTokenResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        event_builder.map(|i| i.set_response_body(&response));
        router_env::logger::info!(connector_response=?response);
        RouterData::try_from(ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        })
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }
```

- `get_error_response` method to manage error responses. As the handling of checkout errors remains consistent across various flows, we've incorporated it from the `build_error_response` method within the `ConnectorCommon` trait.

```rust
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
```

### ConnectorCommonExt
Adds functions with a generic type, including the `build_headers` method. This method constructs both common headers and Authorization headers (from `get_auth_header`), returning them as a vector.

```rust
    impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for Billwerk
    where
        Self: ConnectorIntegration<Flow, Request, Response>,
    {
        fn build_headers(
            &self,
            req: &RouterData<Flow, Request, Response>,
            _connectors: &Connectors,
        ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
            let mut header = vec![(
                headers::CONTENT_TYPE.to_string(),
                self.get_content_type().to_string().into(),
            )];
            let mut api_key = self.get_auth_header(&req.connector_auth_type)?;
            header.append(&mut api_key);
            Ok(header)
        }
    }
```

### OtherTraits
**Payment :** This trait includes several other traits and is meant to represent the functionality related to payments.

**PaymentAuthorize :** This trait extends the `api::ConnectorIntegration `trait with specific types related to payment authorization.

**PaymentCapture :** This trait extends the `api::ConnectorIntegration `trait with specific types related to manual payment capture.

**PaymentSync :** This trait extends the `api::ConnectorIntegration `trait with specific types related to payment retrieve.

**Refund :** This trait includes several other traits and is meant to represent the functionality related to Refunds.

**RefundExecute :** This trait extends the `api::ConnectorIntegration `trait with specific types related to refunds create.

**RefundSync :** This trait extends the `api::ConnectorIntegration `trait with specific types related to refunds retrieve.

And the below derive traits

- **Debug**
- **Clone**
- **Copy**

### **Set the currency Unit**

Part of the `ConnectorCommon` trait, it allows connectors to specify their accepted currency unit as either `Base` or `Minor`. For example, PayPal uses the base unit (e.g., USD), while Hyperswitch uses the minor unit (e.g., cents). Conversion is required if the connector uses the base unit.

```rust
impl<T>
    TryFrom<(
        &types::api::CurrencyUnit,
        types::storage::enums::Currency,
        i64,
        T,
    )> for PaypalRouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        (currency_unit, currency, amount, item): (
            &types::api::CurrencyUnit,
            types::storage::enums::Currency,
            i64,
            T,
        ),
    ) -> Result<Self, Self::Error> {
        let amount = utils::get_amount_as_string(currency_unit, amount, currency)?;
        Ok(Self {
            amount,
            router_data: item,
        })
    }
}
```

### **Connector utility functions**

Contains utility functions for constructing connector requests and responses. Use these helpers for retrieving fields like `get_billing_country`, `get_browser_info`, and `get_expiry_date_as_yyyymm`, as well as for validations like `is_three_ds` and `is_auto_capture`.

```rust
  let json_wallet_data: CheckoutGooglePayData = wallet_data.get_wallet_token_as_json()?;
```

### **Connector configs for control center**

This section is for developers using the [Hyperswitch Control Center](https://github.com/juspay/hyperswitch-control-center). Update the connector configuration in development.toml and run the wasm-pack build command. Replace placeholders with actual paths.

1. Install wasm-pack:

```bash 
cargo install wasm-pack
```

2. Add connector configuration:

    Open the development.toml file located at crates/connector_configs/toml/development.toml in your Hyperswitch project.
    Find the [stripe] section and add the configuration for example_connector. Example:

    ```toml
   # crates/connector_configs/toml/development.toml

   # Other connector configurations...

   [stripe]
   [stripe.connector_auth.HeaderKey]
   api_key="Secret Key"

   # Add any other Stripe-specific configuration here

   [example_connector]
   # Your specific connector configuration for reference
   # ...

   ```

3. Update paths:
    Replace /absolute/path/to/hyperswitch-control-center and /absolute/path/to/hyperswitch with actual paths.

4. Run `wasm-pack` build:
    wasm-pack build --target web --out-dir /absolute/path/to/hyperswitch-control-center/public/hyperswitch/wasm --out-name euclid /absolute/path/to/hyperswitch/crates/euclid_wasm -- --features dummy_connector

By following these steps, you should be able to update the connector configuration and build the WebAssembly files successfully.

### Update `ConnectorTypes.res` and `ConnectorUtils.res`

1. **Update `ConnectorTypes.res`**:
   - Open `src/screens/HyperSwitch/Connectors/ConnectorTypes.res`.
   - Add your connector to the `connectorName` enum:
     ```reason
     type connectorName =
       | Stripe
       | DummyConnector
       | YourNewConnector
     ```
   - Save the file.

2. **Update `ConnectorUtils.res`**:
   - Open `src/screens/HyperSwitch/Connectors/ConnectorUtils.res`.
   - Update functions with your connector:
     ```reason
     let connectorList : array<connectorName> = [Stripe, YourNewConnector]

     let getConnectorNameString = (connectorName: connectorName) =>
       switch connectorName {
       | Stripe => "Stripe"
       | YourNewConnector => "Your New Connector"
       };

     let getConnectorInfo = (connectorName: connectorName) =>
       switch connectorName {
       | Stripe => "Stripe description."
       | YourNewConnector => "Your New Connector description."
       };
     ```
   - Save the file.

### Add Connector Icon

1. **Prepare the Icon**:  
   Name your connector icon in uppercase (e.g., `YOURCONNECTOR.SVG`).

2. **Add the Icon**:  
   Navigate to `public/hyperswitch/Gateway` and copy your SVG icon file there.

3. **Verify Structure**:  
   Ensure the file is correctly placed in `public/hyperswitch/Gateway`:

   ```
   public
   └── hyperswitch
       └── Gateway
           └── YOURCONNECTOR.SVG
   ```
   Save the changes made to the `Gateway` folder.

### **Test the Connector**

1. **Template Code**  

   The template script generates a test file with 20 sanity tests. Implement these tests when adding a new connector.

   Example test:
   ```rust
    #[serial_test::serial]
    #[actix_web::test]
    async fn should_only_authorize_payment() {
        let response = CONNECTOR
            .authorize_payment(payment_method_details(), get_default_payment_info())
            .await
            .expect("Authorize payment response");
        assert_eq!(response.status, enums::AttemptStatus::Authorized);
    }
   ```

2. **Utility Functions** 

    Helper functions for tests are available in `tests/connector/utils`, making test writing easier.

3. **Set API Keys**

    Before running tests, configure API keys in sample_auth.toml and set the environment variable:

    ```bash
    export CONNECTOR_AUTH_FILE_PATH="/hyperswitch/crates/router/tests/connectors/sample_auth.toml"
    cargo test --package router --test connectors -- checkout --test-threads=1
    ```

### **Build Payment Request and Response from JSON Schema**

1. **Install OpenAPI Generator:**
   ```bash
   brew install openapi-generator
   ```

2. **Generate Rust Code:**
    ```bash
    export CONNECTOR_NAME="<CONNECTOR-NAME>"
    export SCHEMA_PATH="<PATH-TO-SCHEMA>"
    openapi-generator generate -g rust -i ${SCHEMA_PATH} -o temp && cat temp/src/models/* > crates/router/src/connector/${CONNECTOR_NAME}/temp.rs && rm -rf temp && sed -i'' -r "s/^pub use.*//;s/^pub mod.*//;s/^\/.*//;s/^.\*.*//;s/crate::models:://g;" crates/router/src/connector/${CONNECTOR_NAME}/temp.rs && cargo +nightly fmt
    ```