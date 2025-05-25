# Hyperswitch Connector Integration: Step-by-Step Guide

This guide provides a reusable, step-by-step process for accurately adding a new payment connector to the Hyperswitch system. It synthesizes concrete implementation patterns from existing connectors and the Hyperswitch architecture to ensure consistent, maintainable integrations.

## Part 1: Overview and Manual Preparation

### Introduction
Integrating a new connector involves understanding its API, mapping it to Hyperswitch's architecture, implementing data transformations, writing core logic, configuring the system, and thorough testing. This guide outlines a methodical approach to this process.

### Prerequisites
-   **Connector API Knowledge**: Thoroughly understand the target connector's API documentation (endpoints, request/response formats, authentication, error codes). This information will be crucial for populating the `technical-spec.md`.
-   **Sandbox Credentials**: Obtain API credentials (API keys, secrets, etc.) for the connector's sandbox/testing environment. These will be documented in `technical-spec.md` and used for `sample_auth.toml`.
-   **Rust Environment**: Ensure Rust nightly toolchain is installed.

### Recommended Standard Imports for Connector Development
When developing the connector, particularly in `{{connector-name-lowercase}}/transformers.rs` and `{{connector-name-lowercase}}.rs`, include these standard imports as needed:

```rust
// Std / Built-in
use std::fmt::Debug;
use time::PrimitiveDateTime;
// use uuid::Uuid; // Uncomment if UUIDs are directly used

// External Crates
// use base64::Engine; // Uncomment if direct base64 encoding/decoding is needed
use masking::{ExposeInterface, PeekInterface, Secret};
use serde::{Deserialize, Serialize};
// use url::Url; // Uncomment if URL parsing/construction is complex

// Common/Internal Utilities
use common_enums::{enums, enums::AuthenticationType, Currency}; // enums::CountryAlpha2 etc.
use common_utils::{
    // consts::{self, BASE64_ENGINE}, // Uncomment if BASE64_ENGINE is used
    date_time,
    errors::CustomResult,
    ext_traits::ValueExt, // For .parse_struct() on responses
    pii::{self, Email}, // pii::IpAddress etc.
    request::Method,
    types::{MinorUnit, StringMajorUnit, StringMinorUnit},
};

// Project Modules - Domain Models
use hyperswitch_domain_models::{
    payment_method_data::{
        BankDebitData, BankRedirectData, BankTransferData, Card, CardRedirectData, GiftCardData,
        PayLaterData, PaymentMethodData, VoucherData, WalletData,
    },
    router_data::{
        AccessToken, AdditionalPaymentMethodConnectorResponse, ConnectorAuthType,
        ConnectorResponseData, ErrorResponse, KlarnaSdkResponse, PaymentMethodToken, RouterData,
    },
    router_flow_types::{
        access_token_auth::AccessTokenAuth, // Example flow
        payments::{Authorize, Capture, PostSessionTokens, PreProcessing, Sync}, // Common payment flows
        refunds::{Execute, RSync}, // Common refund flows
        VerifyWebhookSource,
        // #[cfg(feature = "payouts")] // Uncomment if payouts are implemented
        // PoFulfill,
    },
    router_request_types::{
        AccessTokenRequestData, BrowserInformation, CompleteAuthorizeData, CustomerDetails, MandateRevokeRequestData,
        PaymentMethodTokenizationData, PaymentsAuthorizeData, PaymentsCancelData,
        PaymentsCaptureData, PaymentsPostSessionTokensData, PaymentsPreProcessingData,
        PaymentsSetupMandateRequestData, PaymentsSyncData, ResponseId,
        SetupMandateRequestData, VerifyWebhookSourceRequestData,
    },
    router_response_types::{
        MandateReference, PaymentsResponseData, PayoutsResponseData, RedirectForm,
        RefundsResponseData, VerifyWebhookSourceResponseData, VerifyWebhookStatus,
    },
    types::{ // These are often the target types for TryFrom implementations
        PaymentsAuthorizeRouterData, PaymentsCancelRouterData, PaymentsCaptureRouterData,
        PaymentsCompleteAuthorizeRouterData, PaymentsPostSessionTokensRouterData,
        PaymentsPreProcessingRouterData, RefreshTokenRouterData, RefundsRouterData,
        SdkSessionUpdateRouterData, SetupMandateRouterData, VerifyWebhookSourceRouterData,
        // AccessTokenRouterData, // if defined
    },
};

// Project Modules - Interfaces
use hyperswitch_interfaces::{api as connector_api, consts as hs_consts, errors}; // Renamed to avoid conflicts

// API Models (from api_models crate)
use api_models::{
    enums as api_enums, // api_models::enums::CountryAlpha2, etc.
    payments::{KlarnaSessionTokenResponse, SessionToken},
    webhooks::IncomingWebhookEvent,
    // #[cfg(feature = "payouts")] // Uncomment if payouts are implemented
    // payouts::{PayoutMethodData, Wallet as WalletPayout},
};

// Crate (local module, i.e., hyperswitch_connectors) imports
use crate::{
    constants, // Connector-specific constants if any
    types::{ // Wrappers like ResponseRouterData, etc.
        PaymentsCaptureResponseRouterData, PaymentsResponseRouterData,
        PaymentsSessionResponseRouterData, PayoutsResponseRouterData, RefundsResponseRouterData,
        ResponseRouterData, {{CONNECTOR_PASCAL_CASE}}RouterData, // If using the amount conversion wrapper
    },
    unimplemented_payment_method,
    utils::{ // Connector utils, including request data accessor traits
        self, missing_field_err, to_connector_meta, to_connector_meta_from_secret,
        AccessTokenRequestInfo, AddressData, AddressDetailsData, BrowserInformationData, CardData,
        CardData as CardDataUtil, ForeignTryFrom, PaymentMethodTokenizationRequestData,
        PaymentsAuthorizeRequestData, PaymentsCompleteAuthorizeRequestData,
        PaymentsPostSessionTokensRequestData, PaymentsPreProcessingRequestData,
        PaymentsSetupMandateRequestData, PaymentsSyncRequestData, RouterData as _, // For trait methods on RouterData
    },
};
```

## Part 2: Generating Connector-Specific Plans

### Step 1: Generate Connector Template (Manual/Scripted)
-   **Action**: Run the `add_connector.sh` script.
    ```bash
    sh scripts/add_connector.sh {{connector-name-lowercase}} {{connector-base-url}}
    ```
    (Replace placeholders with actual connector name and its base API URL)
-   **Verify Output Structure**:
    -   `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs`
    -   `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs` (main logic)
    -   `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/test.rs`
-   **Manual File Move (Test File)**:
    -   From: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/test.rs`
    -   To: `crates/router/tests/connectors/{{connector-name-lowercase}}.rs`

### Step 2: Create Technical Specification
-   **Action**: Create and populate a detailed technical specification document:
    -   Create `connector_integration/{{connector-name-lowercase}}/tech-spec.md`
    -   Document connector API endpoints, authentication mechanisms, request/response formats, and flow mappings
    -   Define struct fields and type mappings from connector to Hyperswitch
    -   Include amount handling details (base unit vs. minor unit)
    -   Document error handling and status mappings

## Part 3: Core Integration (Implementation)

### Phase A: Authentication & Error Handling

#### Step 3: Implement Authentication (`transformers.rs`)
-   **Action**: Define `{{CONNECTOR_PASCAL_CASE}}AuthType` and implement `TryFrom<&ConnectorAuthType>`.
    ```rust
    // Example for API key in header
    pub struct {{CONNECTOR_PASCAL_CASE}}AuthType {
        pub(super) api_key: Secret<String>,
    }

    impl TryFrom<&ConnectorAuthType> for {{CONNECTOR_PASCAL_CASE}}AuthType {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
            match auth_type {
                ConnectorAuthType::HeaderKey { api_key } => Ok(Self {
                    api_key: api_key.to_owned(),
                }),
                _ => Err(errors::ConnectorError::FailedToObtainAuthType.into()),
            }
        }
    }
    ```

#### Step 4: Implement Error Handling (`transformers.rs` & `{{connector-name-lowercase}}.rs`)
-   **Action**: Define error response structure and build error response implementation.
    ```rust
    // In transformers.rs
    #[derive(Debug, Deserialize, Serialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}ErrorResponse {
        pub code: String,
        pub message: String,
        pub reason: Option<String>,
    }

    // In {{connector-name-lowercase}}.rs
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: {{CONNECTOR_PASCAL_CASE}}ErrorResponse = res
            .response
            .parse_struct("{{CONNECTOR_PASCAL_CASE}}ErrorResponse")
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
            network_error_message: None,
            network_decline_code: None,
            network_advice_code: None,
        })
    }
    ```

### Phase B: Common Connector Implementation

#### Step 5: Implement `ConnectorCommon` & Setup Traits (`{{connector-name-lowercase}}.rs`)
-   **Action**: Implement the `ConnectorCommon` trait with connector-specific values.
    ```rust
    impl ConnectorCommon for {{CONNECTOR_PASCAL_CASE}} {
        fn id(&self) -> &'static str {
            "{{connector-name-lowercase}}"
        }

        fn get_currency_unit(&self) -> api::CurrencyUnit {
            // Based on connector's amount format (base unit vs. minor unit)
            api::CurrencyUnit::Base // or api::CurrencyUnit::Minor
        }

        fn common_get_content_type(&self) -> &'static str {
            "application/json"
        }

        fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
            connectors.{{connector-name-lowercase}}.base_url.as_ref()
        }

        fn get_auth_header(
            &self,
            auth_type: &ConnectorAuthType,
        ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
            let auth = {{connector-name-lowercase}}::{{CONNECTOR_PASCAL_CASE}}AuthType::try_from(auth_type)
                .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
            Ok(vec![(
                headers::AUTHORIZATION.to_string(),
                format!("Bearer {}", auth.api_key.expose()).into_masked(),
            )])
        }
    }
    ```

-   **Action**: Define connector trait implementations.
    ```rust
    impl api::Payment for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::PaymentToken for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::PaymentAuthorize for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::PaymentVoid for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::MandateSetup for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::ConnectorAccessToken for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::PaymentSync for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::PaymentCapture for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::PaymentSession for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::Refund for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::RefundExecute for {{CONNECTOR_PASCAL_CASE}} {}
    impl api::RefundSync for {{CONNECTOR_PASCAL_CASE}} {}
    ```

#### Step 6: Implement `ConnectorCommonExt` for Header Building (`{{connector-name-lowercase}}.rs`)
-   **Action**: Implement the header building logic.
    ```rust
    impl<Flow, Request, Response> ConnectorCommonExt<Flow, Request, Response> for {{CONNECTOR_PASCAL_CASE}}
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

### Phase C: Implement Amount Conversion Helper (If Needed)

#### Step 7: Define Router Data Wrapper (`transformers.rs`)
-   **Action**: If the connector requires amount conversion, define a router data wrapper.
    ```rust
    pub struct {{CONNECTOR_PASCAL_CASE}}RouterData<T> {
        pub amount: f64, // or StringMinorUnit, StringMajorUnit, etc.
        pub router_data: T,
    }

    impl<T> TryFrom<(&api::CurrencyUnit, enums::Currency, i64, T)> for {{CONNECTOR_PASCAL_CASE}}RouterData<T> {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(
            (currency_unit, currency, amount, item): (&api::CurrencyUnit, enums::Currency, i64, T),
        ) -> Result<Self, Self::Error> {
            let amount = utils::get_amount_as_f64(currency_unit, amount, currency)?;
            Ok(Self {
                amount,
                router_data: item,
            })
        }
    }
    ```

### Phase D: Payment Flow Implementation - Authorize Flow Example

#### Step 8: Define Payment Request & Response Structs (`transformers.rs`)
-   **Action**: Create structs representing the connector's request/response formats.
    ```rust
    // Request struct
    #[derive(Default, Debug, Serialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}PaymentsRequest {
        amount: f64,
        currency: String,
        order_id: String,
        card: {{CONNECTOR_PASCAL_CASE}}Card,
        // Other fields as required by the connector API
    }

    #[derive(Default, Debug, Serialize, Eq, PartialEq)]
    pub struct {{CONNECTOR_PASCAL_CASE}}Card {
        number: cards::CardNumber,
        expiry_month: Secret<String>,
        expiry_year: Secret<String>,
        cvv: Secret<String>,
    }

    // Response struct
    #[derive(Default, Debug, Clone, Deserialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}PaymentsResponse {
        id: String,
        status: {{CONNECTOR_PASCAL_CASE}}PaymentStatus,
        amount: f64,
        currency: String,
        // Other fields from the connector response
    }

    // Status enum
    #[derive(Debug, Clone, Deserialize)]
    #[serde(rename_all = "snake_case")]
    pub enum {{CONNECTOR_PASCAL_CASE}}PaymentStatus {
        Authorized,
        Captured,
        Failed,
        Pending,
        #[serde(other)]
        Unknown,
    }

    impl From<{{CONNECTOR_PASCAL_CASE}}PaymentStatus> for enums::AttemptStatus {
        fn from(status: {{CONNECTOR_PASCAL_CASE}}PaymentStatus) -> Self {
            match status {
                {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Authorized => Self::Authorized,
                {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Captured => Self::Charged,
                {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Failed => Self::Failure,
                {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Pending => Self::Pending,
                {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Unknown => Self::Pending,
            }
        }
    }
    ```

#### Step 9: Implement TryFrom for Request Conversion (`transformers.rs`)
-   **Action**: Implement conversion from Hyperswitch request data to connector request format.
    ```rust
    impl TryFrom<&{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsAuthorizeRouterData>> for {{CONNECTOR_PASCAL_CASE}}PaymentsRequest {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: &{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsAuthorizeRouterData>) -> Result<Self, Self::Error> {
            match item.router_data.request.payment_method_data.clone() {
                PaymentMethodData::Card(req_card) => {
                    let card = {{CONNECTOR_PASCAL_CASE}}Card {
                        number: req_card.card_number,
                        expiry_month: req_card.card_exp_month,
                        expiry_year: req_card.card_exp_year,
                        cvv: req_card.card_cvc,
                    };

                    Ok(Self {
                        amount: item.amount,
                        currency: item.router_data.request.currency.to_string(),
                        order_id: item.router_data.connector_request_reference_id.clone(),
                        card,
                    })
                }
                _ => Err(errors::ConnectorError::NotImplemented(
                    utils::get_unimplemented_payment_method_error_message("{{connector-name-lowercase}}"),
                ).into()),
            }
        }
    }
    ```

#### Step 10: Implement TryFrom for Response Conversion (`transformers.rs`)
-   **Action**: Implement conversion from connector response to Hyperswitch response data.
    ```rust
    impl<F> TryFrom<ResponseRouterData<F, {{CONNECTOR_PASCAL_CASE}}PaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>>
        for RouterData<F, PaymentsAuthorizeData, PaymentsResponseData>
    {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(
            item: ResponseRouterData<F, {{CONNECTOR_PASCAL_CASE}}PaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>,
        ) -> Result<Self, Self::Error> {
            let connector_response = item.response;
            let attempt_status = enums::AttemptStatus::from(connector_response.status);
            
            Ok(Self {
                status: attempt_status,
                response: Ok(PaymentsResponseData::TransactionResponse {
                    resource_id: ResponseId::ConnectorTransactionId(connector_response.id),
                    redirection_data: Box::new(None),
                    mandate_reference: Box::new(None),
                    connector_metadata: None,
                    network_txn_id: None,
                    connector_response_reference_id: None,
                    incremental_authorization_allowed: None,
                    charges: None,
                }),
                ..item.data
            })
        }
    }
    ```

#### Step 11: Implement Authorize Flow (`{{connector-name-lowercase}}.rs`)
-   **Action**: Implement the `ConnectorIntegration` trait for the Authorize flow.
    ```rust
    impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for {{CONNECTOR_PASCAL_CASE}} {
        fn get_headers(
            &self,
            req: &PaymentsAuthorizeRouterData,
            connectors: &Connectors,
        ) -> CustomResult<Vec<(String, masking::Maskable<String>)>, errors::ConnectorError> {
            self.build_headers(req, connectors)
        }

        fn get_content_type(&self) -> &'static str {
            self.common_get_content_type()
        }

        fn get_url(
            &self,
            _req: &PaymentsAuthorizeRouterData,
            connectors: &Connectors,
        ) -> CustomResult<String, errors::ConnectorError> {
            Ok(format!("{}/payments", self.base_url(connectors)))
        }

        fn get_request_body(
            &self,
            req: &PaymentsAuthorizeRouterData,
            _connectors: &Connectors,
        ) -> CustomResult<RequestContent, errors::ConnectorError> {
            let connector_router_data = {{CONNECTOR_PASCAL_CASE}}RouterData::try_from((
                &self.get_currency_unit(),
                req.request.currency,
                req.request.amount,
                req,
            ))?;

            let connector_req = {{CONNECTOR_PASCAL_CASE}}PaymentsRequest::try_from(&connector_router_data)?;
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
                    .url(&PaymentsAuthorizeType::get_url(self, req, connectors)?)
                    .attach_default_headers()
                    .headers(PaymentsAuthorizeType::get_headers(self, req, connectors)?)
                    .set_body(PaymentsAuthorizeType::get_request_body(self, req, connectors)?)
                    .build(),
            ))
        }

        fn handle_response(
            &self,
            data: &PaymentsAuthorizeRouterData,
            event_builder: Option<&mut ConnectorEvent>,
            res: Response,
        ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
            let response: {{CONNECTOR_PASCAL_CASE}}PaymentsResponse = res
                .response
                .parse_struct("{{CONNECTOR_PASCAL_CASE}}PaymentsResponse")
                .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
                
            event_builder.map(|i| i.set_response_body(&response));
            router_env::logger::info!(connector_response=?response);

            RouterData::try_from(ResponseRouterData {
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
    }
    ```

### Phase E: Additional Payment Flow Implementations

Follow the pattern from Phase D to implement other required flows:
- Payment Sync (PSync)
- Capture
- Refund (Execute)
- Refund Sync (RSync)
- Void/Cancel

## Part 4: Finalization and Testing

### Step 12: Implement Connector Specifications (`{{connector-name-lowercase}}.rs`)
-   **Action**: Define connector metadata and supported features.
    ```rust
    lazy_static! {
        static ref {{CONNECTOR_PASCAL_CASE}}_SUPPORTED_PAYMENT_METHODS: SupportedPaymentMethods = {
            let default_capture_methods = vec![
                enums::CaptureMethod::Automatic,
                enums::CaptureMethod::Manual,
            ];

            let supported_card_network = vec![
                common_enums::CardNetwork::Visa,
                common_enums::CardNetwork::Mastercard,
                common_enums::CardNetwork::AmericanExpress,
                // Add other supported networks
            ];

            let mut supported_payment_methods = SupportedPaymentMethods::new();

            supported_payment_methods.add(
                enums::PaymentMethod::Card,
                enums::PaymentMethodType::Credit,
                PaymentMethodDetails {
                    mandates: common_enums::FeatureStatus::NotSupported,
                    refunds: common_enums::FeatureStatus::Supported,
                    supported_capture_methods: default_capture_methods.clone(),
                    specific_features: Some(
                        api_models::feature_matrix::PaymentMethodSpecificFeatures::Card({
                            api_models::feature_matrix::CardSpecificFeatures {
                                three_ds: common_enums::FeatureStatus::Supported,
                                no_three_ds: common_enums::FeatureStatus::Supported,
                                supported_card_networks: supported_card_network.clone(),
                            }
                        }),
                    ),
                },
            );

            // Add other payment methods if supported

            supported_payment_methods
        };
        
        static ref {{CONNECTOR_PASCAL_CASE}}_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
            display_name: "{{CONNECTOR_NAME}}",
            description: "Description of {{CONNECTOR_NAME}} payment service.",
            connector_type: enums::PaymentConnectorCategory::PaymentGateway,
        };
        
        static ref {{CONNECTOR_PASCAL_CASE}}_SUPPORTED_WEBHOOK_FLOWS: Vec<enums::EventClass> = vec![
            // List supported webhook event classes
            // enums::EventClass::Payment,
            // enums::EventClass::Refund,
        ];
    }

    impl ConnectorSpecifications for {{CONNECTOR_PASCAL_CASE}} {
        fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
            Some(&*{{CONNECTOR_PASCAL_CASE}}_CONNECTOR_INFO)
        }

        fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
            Some(&*{{CONNECTOR_PASCAL_CASE}}_SUPPORTED_PAYMENT_METHODS)
        }

        fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
            Some(&*{{CONNECTOR_PASCAL_CASE}}_SUPPORTED_WEBHOOK_FLOWS)
        }
    }
    ```

### Step 13: Implement Webhook Handling (If Required)
-   **Action**: Implement the `IncomingWebhook` trait if the connector supports webhooks.
    ```rust
    #[async_trait::async_trait]
    impl webhooks::IncomingWebhook for {{CONNECTOR_PASCAL_CASE}} {
        fn get_webhook_object_reference_id(
            &self,
            request: &webhooks::IncomingWebhookRequestDetails<'_>,
        ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
            let webhook_body: {{CONNECTOR_PASCAL_CASE}}WebhookBody = request
                .body
                .parse_struct("{{CONNECTOR_PASCAL_CASE}}WebhookBody")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
                
            // Extract and return the appropriate reference ID based on event type
            // Example:
            // Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            //     webhook_body.data.payment_id,
            // ))
        }

        fn get_webhook_event_type(
            &self,
            request: &webhooks::IncomingWebhookRequestDetails<'_>,
        ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
            let webhook_body: {{CONNECTOR_PASCAL_CASE}}WebhookBody = request
                .body
                .parse_struct("{{CONNECTOR_PASCAL_CASE}}WebhookBody")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
                
            // Map connector event type to Hyperswitch event type
            // Example:
            // match webhook_body.event_type.as_str() {
            //     "payment.succeeded" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess),
            //     "payment.failed" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure),
            //     _ => Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported),
            // }
        }

        fn get_webhook_resource_object(
            &self,
            request: &webhooks::IncomingWebhookRequestDetails<'_>,
        ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
            let webhook_body: {{CONNECTOR_PASCAL_CASE}}WebhookBody = request
                .body
                .parse_struct("{{CONNECTOR_PASCAL_CASE}}WebhookBody")
                .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
                
            // Return the parsed webhook object
            // Example:
            // Ok(Box::new(webhook_body.data))
        }
    }
    ```

### Step 14: Register Connector in Enum Files
-   **Action**: Update `crates/common_enums/src/connector_enums.rs` to include the new connector.
    ```rust
    // Add to Connector enum
    pub enum Connector {
        // ...
        {{CONNECTOR_PASCAL_CASE}},
        // ...
    }

    // Add to RoutableConnectors enum
    pub enum RoutableConnectors {
        // ...
        {{CONNECTOR_PASCAL_CASE}},
        // ...
    }

    // Update From implementation for Connector
    impl From<String> for Connector {
        fn from(connector: String) -> Self {
            match connector.as_str() {
                // ...
                "{{connector-name-lowercase}}" => Self::{{CONNECTOR_PASCAL_CASE}},
                // ...
                _ => Self::DummyConnector1,
            }
        }
    }

    // Update similar TryFrom implementation
    impl TryFrom<&str> for RoutableConnectors {
        type Error = errors::ConnectorError;
        fn try_from(connector: &str) -> Result<Self, Self::Error> {
            match connector {
                // ...
                "{{connector-name-lowercase}}" => Ok(Self::{{CONNECTOR_PASCAL_CASE}}),
                // ...
                _ => Err(errors::ConnectorError::NotImplemented(format!(
                    "Connector {} not implemented",
                    connector
                ))),
            }
        }
    }
    ```

### Step 15: Add Configuration in `development.toml`
-   **Action**: Add configuration for the new connector in `crates/connector_configs/toml/development.toml`.
    ```toml
    [{{connector-name-lowercase}}]
    base_url = "https://api.sandbox.{{connector-name-lowercase}}.com" # Use the actual sandbox URL
    # Add any other connector-specific configs needed

    # Choose the appropriate auth type based on the connector's requirements
    [{{connector-name-lowercase}}.connector_auth.HeaderKey]
    api_key = "env_var_for_api_key" # Variable that will be read from environment

    # Other auth types:
    # [{{connector-name-lowercase}}.connector_auth.BodyKey]
    # api_key = "env_var_for_api_key"
    # key1 = "env_var_for_key1"

    # [{{connector-name-lowercase}}.connector_auth.SignatureKey]
    # api_key = "env_var_for_api_key"
    # key1 = "env_var_for_key1"
    # key2 = "env_var_for_key2"
    ```

### Step 16: Add Test Authentication in `sample_auth.toml`
-   **Action**: Add test credentials in `crates/router/tests/connectors/sample_auth.toml`.
    ```toml
    [{{connector-name-lowercase}}]
    # Enter actual sandbox credentials (values, not environment variable names)
    api_key = "sandbox_api_key"
    # key1 = "sandbox_key1"
    # key2 = "sandbox_key2" 
    ```

### Step 17: Implement and Run Tests
-   **Action**: Update the test file at `crates/router/tests/connectors/{{connector-name-lowercase}}.rs`.
    1. Implement the test struct with appropriate data:
       ```rust
       struct {{CONNECTOR_PASCAL_CASE}}Test;

       impl ConnectorTest for {{CONNECTOR_PASCAL_CASE}}Test {
           fn get_connector_name(&self) -> &str {
               "{{connector-name-lowercase}}"
           }

           fn get_auth_token(&self) -> &str {
               "api_key" // Main auth field name from your config
           }

           fn get_payment_info(&self) -> Option<PaymentInfo> {
               Some(PaymentInfo {
                   payment_method_types: vec!["card".to_string()],
                   ..Default::default()
               })
           }

           fn get_payment_method_details(&self) -> Option<PaymentMethodDetails> {
               Some(PaymentMethodDetails::Card(CardDetails {
                   card_number: cards::CardNumber::from_str("4242424242424242").unwrap(),
                   card_exp_month: Secret::new("02".to_string()),
                   card_exp_year: Secret::new("2035".to_string()),
                   card_cvc: Secret::new("123".to_string()),
                   card_holder_name: Secret::new("John Doe".to_string()),
                   ..Default::default()
               }))
           }
       }
       ```

    2. Run the tests with the following command:
       ```bash
       export CONNECTOR_AUTH_FILE_PATH="/path/to/hyperswitch/crates/router/tests/connectors/sample_auth.toml"
       cargo test --package router --test connectors -- {{connector-name-lowercase}} --test-threads=1
       ```

## Conclusion

This guide provides a comprehensive approach to integrating a new payment connector into Hyperswitch. It covers:

1. **Preparation**: Understanding the connector API and setting up the environment
2. **Implementation**: 
   - Creating the connector template
   - Implementing authentication and error handling
   - Setting up the basic connector traits
   - Implementing payment flows (Authorize, Capture, Refund, etc.)
   - Adding connector specifications
3. **Configuration and Testing**: 
   - Registering the connector in the enum files
   - Adding configuration in development.toml
   - Setting up test authentication
   - Implementing and running tests

By following this structured approach, you can ensure a consistent, maintainable, and reliable connector integration.

Remember to thoroughly test each payment flow using both the automated tests and manual testing with real sandbox accounts to verify the integration works correctly in all scenarios.
