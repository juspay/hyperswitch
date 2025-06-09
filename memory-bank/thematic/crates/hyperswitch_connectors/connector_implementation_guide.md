---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

# Connector Implementation Guide

---
**Parent:** [Hyperswitch Connectors Overview](./overview.md)  
**Related Files:**
- [Connector Interface Requirements](./connector_interface_guide.md)
- [Connector Testing Guide](./connector_testing_guide.md)
- [Connector Configuration Guide](./connector_configuration_guide.md)
---

## Overview

This guide provides detailed, step-by-step instructions for implementing a new payment connector in Hyperswitch. It covers the entire implementation process, from initial setup to final testing and integration.

## Prerequisites

Before implementing a new connector, you should:

1. Have a thorough understanding of the [Connector Interface Requirements](./connector_interface_guide.md)
2. Familiarize yourself with the payment processor's API documentation
3. Obtain API credentials for testing with the payment processor
4. Understand the Hyperswitch domain models and routing architecture

## Implementation Steps

### 1. Setup Project Structure

Start by creating the necessary files for your connector implementation:

```
hyperswitch_connectors/
└── src/
    └── connectors/
        └── your_connector_name/
            ├── mod.rs            # Main connector implementation
            ├── transformers.rs    # Data transformation logic
            ├── utils.rs           # Connector-specific utilities
            └── tests/             # Unit tests for the connector
                └── mod.rs         # Test module
```

Create these files and follow the template structure provided below.

### 2. Implement Connector Types

In the `transformers.rs` file, define the connector-specific data types needed for request/response transformations:

```rust
// transformers.rs
use std::collections::HashMap;

use common_enums::{enums, AttemptStatus};
use common_utils::request::Method;
use hyperswitch_domain_models::{...};
use hyperswitch_interfaces::{api::CurrencyUnit, errors};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{types::{RefundsResponseRouterData, ResponseRouterData}, utils::PaymentsAuthorizeRequestData};

// Request Types
#[derive(Default, Debug, Serialize)]
pub struct YourConnectorPaymentsRequest {
    // Define fields according to the connector's API
    pub amount: i64,
    pub currency: String,
    pub description: String,
    // Add other required fields
}

// Response Types
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct YourConnectorPaymentsResponse {
    // Define fields according to the connector's API
    pub id: String,
    pub status: YourConnectorPaymentStatus,
    // Add other fields from the response
}

// Status Enum
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum YourConnectorPaymentStatus {
    Success,
    Pending,
    Failed,
    #[default]
    Processing,
    #[serde(other)]
    Unknown,
}

// Map connector status to Hyperswitch status
impl From<YourConnectorPaymentStatus> for AttemptStatus {
    fn from(item: YourConnectorPaymentStatus) -> Self {
        match item {
            YourConnectorPaymentStatus::Success => Self::Charged,
            YourConnectorPaymentStatus::Pending => Self::Pending,
            YourConnectorPaymentStatus::Failed => Self::Failure,
            YourConnectorPaymentStatus::Processing => Self::Pending,
            YourConnectorPaymentStatus::Unknown => Self::Pending,
        }
    }
}

// Auth Type
pub struct YourConnectorAuthType {
    pub(super) api_key: Secret<String>,
    // Add other auth fields if needed
}

// Implement auth type conversion
impl TryFrom<&ConnectorAuthType> for YourConnectorAuthType {
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

// Request Transformer
impl TryFrom<&PaymentsAuthorizeRouterData> for YourConnectorPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        let amount = item.request.amount;
        let currency = item.request.currency.to_string();
        let description = item.get_description()?;
        
        Ok(Self {
            amount,
            currency,
            description,
            // Map other fields
        })
    }
}

// Response Transformer
impl<F, T> TryFrom<ResponseRouterData<F, YourConnectorPaymentsResponse, T, PaymentsResponseData>>
    for RouterData<F, T, PaymentsResponseData>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(
        item: ResponseRouterData<F, YourConnectorPaymentsResponse, T, PaymentsResponseData>,
    ) -> Result<Self, Self::Error> {
        let connector_id = ResponseId::ConnectorTransactionId(item.response.id);
        let attempt_status = item.response.status;
        
        let payments_response_data = PaymentsResponseData::TransactionResponse {
            resource_id: connector_id,
            redirection_data: Box::new(None),  // Add redirection if needed
            mandate_reference: Box::new(None), // Add mandate if needed
            connector_metadata: None,
            network_txn_id: None,
            connector_response_reference_id: None,
            incremental_authorization_allowed: None,
            charges: None,
        };
        
        Ok(Self {
            status: AttemptStatus::from(attempt_status),
            response: Ok(payments_response_data),
            ..item.data
        })
    }
}

// Error Response Type
#[derive(Debug, Deserialize)]
pub struct YourConnectorErrorResponse {
    pub error: String,
    pub message: String,
    // Add other error fields
}
```

### 3. Implement Main Connector Module

In the `mod.rs` file, implement the connector traits and operations:

```rust
// mod.rs
pub mod transformers;

use std::fmt::Debug;

use common_enums::enums;
use common_utils::{errors::CustomResult, request::{Method, Request, RequestBuilder}};
use error_stack::ResultExt;
use hyperswitch_domain_models::...;
use hyperswitch_interfaces::{api, configs::Connectors, consts, errors, events::connector_api_logs::ConnectorEvent, types::{PaymentsAuthorizeType, PaymentsSyncType, Response}};
use masking::Mask;

use self::transformers as your_connector;
use crate::types::ResponseRouterData;

#[derive(Debug, Clone)]
pub struct YourConnector;

// Implement marker traits
impl api::Payment for YourConnector {}
impl api::PaymentSession for YourConnector {}
impl api::PaymentAuthorize for YourConnector {}
impl api::PaymentSync for YourConnector {}
// Add other marker traits as needed

// Implement ConnectorCommon
impl ConnectorCommon for YourConnector {
    fn id(&self) -> &'static str {
        "your_connector"
    }
    
    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }
    
    fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.your_connector.base_url.as_ref()
    }
    
    fn get_currency_unit(&self) -> api::CurrencyUnit {
        api::CurrencyUnit::Minor  // Or Major, depending on the connector
    }
    
    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = your_connector::YourConnectorAuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        
        Ok(vec![("Authorization".to_string(), format!("Bearer {}", auth.api_key.peek()).into())])
    }
    
    fn build_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: your_connector::YourConnectorErrorResponse = res
            .response
            .parse_struct("YourConnectorErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
        
        event_builder.map(|i| i.set_error_response_body(&response));
        
        Ok(ErrorResponse {
            status_code: res.status_code,
            code: consts::NO_ERROR_CODE.to_string(),
            message: response.message,
            reason: None,
            attempt_status: None,
            connector_transaction_id: None,
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
        })
    }
}

// Implement ConnectorIntegration for PaymentAuthorize
impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for YourConnector {
    fn get_headers(
        &self,
        req: &PaymentsAuthorizeRouterData,
        connectors: &Connectors,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
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
        let connector_req = your_connector::YourConnectorPaymentsRequest::try_from(req)?;
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
        let response: your_connector::YourConnectorPaymentsResponse = res
            .response
            .parse_struct("YourConnectorPaymentsResponse")
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
    
    fn get_error_response(
        &self,
        res: Response,
        event_builder: Option<&mut ConnectorEvent>,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res, event_builder)
    }
}

// Implement ConnectorIntegration for PaymentSync
impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData> for YourConnector {
    // Similar to PaymentAuthorize implementation...
}

// Implement ConnectorSpecifications
impl ConnectorSpecifications for YourConnector {
    fn get_connector_about(&self) -> Option<&'static ConnectorInfo> {
        Some(&YOUR_CONNECTOR_INFO)
    }
    
    fn get_supported_payment_methods(&self) -> Option<&'static SupportedPaymentMethods> {
        Some(&*YOUR_CONNECTOR_SUPPORTED_PAYMENT_METHODS)
    }
    
    fn get_supported_webhook_flows(&self) -> Option<&'static [enums::EventClass]> {
        Some(&YOUR_CONNECTOR_SUPPORTED_WEBHOOK_FLOWS)
    }
}

// Define connector metadata
static YOUR_CONNECTOR_SUPPORTED_PAYMENT_METHODS: LazyLock<SupportedPaymentMethods> = LazyLock::new(|| {
    let mut supported_payment_methods = SupportedPaymentMethods::new();
    
    let supported_capture_methods = vec![enums::CaptureMethod::Automatic];
    
    supported_payment_methods.add(
        enums::PaymentMethod::Card,
        enums::PaymentMethodType::Credit,
        PaymentMethodDetails {
            mandates: enums::FeatureStatus::NotSupported,
            refunds: enums::FeatureStatus::Supported,
            supported_capture_methods,
            specific_features: None,
        },
    );
    
    // Add other payment methods...
    
    supported_payment_methods
});

static YOUR_CONNECTOR_INFO: ConnectorInfo = ConnectorInfo {
    display_name: "Your Connector",
    description: "Your connector description",
    connector_type: enums::PaymentConnectorCategory::PaymentGateway,
};

static YOUR_CONNECTOR_SUPPORTED_WEBHOOK_FLOWS: [enums::EventClass; 1] = [enums::EventClass::Payments];
```

### 4. Implement Webhook Handling

If your connector supports webhooks, implement the `IncomingWebhook` trait:

```rust
// In mod.rs

#[async_trait::async_trait]
impl IncomingWebhook for YourConnector {
    fn get_webhook_source_verification_algorithm(
        &self,
        _request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn crypto::VerifySignature + Send>, errors::ConnectorError> {
        Ok(Box::new(crypto::HmacSha256))
    }
    
    fn get_webhook_source_verification_signature(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        let signature = request
            .headers
            .get("X-Signature")
            .ok_or(errors::ConnectorError::WebhookSourceVerificationFailed)?;
        
        hex::decode(signature.as_bytes())
            .change_context(errors::ConnectorError::WebhookSourceVerificationFailed)
    }
    
    fn get_webhook_source_verification_message(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
        _merchant_id: &common_utils::id_type::MerchantId,
        _connector_webhook_secrets: &api_models::webhooks::ConnectorWebhookSecrets,
    ) -> CustomResult<Vec<u8>, errors::ConnectorError> {
        Ok(request.body.to_vec())
    }
    
    fn get_webhook_object_reference_id(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::ObjectReferenceId, errors::ConnectorError> {
        let notif: your_connector::YourConnectorWebhookData = serde_json::from_slice(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        
        Ok(api_models::webhooks::ObjectReferenceId::PaymentId(
            api_models::payments::PaymentIdType::ConnectorTransactionId(notif.id),
        ))
    }
    
    fn get_webhook_event_type(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<api_models::webhooks::IncomingWebhookEvent, errors::ConnectorError> {
        let notif: your_connector::YourConnectorWebhookData = serde_json::from_slice(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        
        match notif.status {
            "success" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentSuccess),
            "failed" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentFailure),
            "pending" => Ok(api_models::webhooks::IncomingWebhookEvent::PaymentIntentProcessing),
            _ => Ok(api_models::webhooks::IncomingWebhookEvent::EventNotSupported),
        }
    }
    
    fn get_webhook_resource_object(
        &self,
        request: &webhooks::IncomingWebhookRequestDetails<'_>,
    ) -> CustomResult<Box<dyn masking::ErasedMaskSerialize>, errors::ConnectorError> {
        let notif: your_connector::YourConnectorWebhookData = serde_json::from_slice(request.body)
            .change_context(errors::ConnectorError::WebhookBodyDecodingFailed)?;
        
        Ok(Box::new(notif))
    }
}
```

### 5. Register the Connector

Register your connector in the connector registry for it to be available in the system:

```rust
// In crates/router/src/connector/connector_registry.rs

// Update the get_connector_integration function
pub fn get_connector_integration(
    connector_name: &str,
) -> CustomResult<Box<dyn api::ConnectorIntegration<api::Payment, types::PaymentsData>>, errors::ApiErrorResponse> {
    let connector: Box<dyn api::ConnectorIntegration<api::Payment, types::PaymentsData>> =
        match connector_name {
            // Existing connectors...
            "stripe" => Box::new(Stripe),
            "adyen" => Box::new(Adyen),
            // Add your new connector
            "your_connector" => Box::new(YourConnector),
            _ => Err(errors::ApiErrorResponse::InvalidConnectorName)?,
        };
    Ok(connector)
}
```

### 6. Add Configuration

Add your connector configuration to the connectors configuration file:

```rust
// In crates/hyperswitch_interfaces/src/configs.rs

#[derive(Debug, Clone, Deserialize)]
pub struct Connectors {
    // Existing connectors...
    pub stripe: ConnectorParams,
    pub adyen: ConnectorParams,
    // Add your connector
    pub your_connector: ConnectorParams,
}
```

Update the connector configuration in the appropriate TOML files:

```toml
# In config/config.example.toml and other configuration files

[connectors.your_connector]
base_url = "https://api.your-connector.com"
```

## Implementation Patterns

### Handling Different Payment Methods

If your connector supports multiple payment methods, you'll need to adapt your transformers based on the payment method:

```rust
// In transformers.rs

impl TryFrom<&PaymentsAuthorizeRouterData> for YourConnectorPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        match &item.request.payment_method_data {
            api_models::payments::PaymentMethodData::Card(card) => {
                // Handle card payment
                // ...
            },
            api_models::payments::PaymentMethodData::BankTransfer(bank_transfer) => {
                // Handle bank transfer
                // ...
            },
            _ => Err(errors::ConnectorError::NotImplemented(
                "Payment method not supported".to_string(),
            ).into()),
        }
    }
}
```

### Handling Redirect Flows

For connectors that require a redirect flow for payment authorization:

```rust
// In transformers.rs, in your ResponseRouterData to RouterData conversion

let redirection_data = RedirectForm::Form {
    endpoint: item.response.redirect_url.to_string(),
    method: Method::Get,
    form_fields: HashMap::new(),
};

let payments_response_data = PaymentsResponseData::TransactionResponse {
    resource_id: connector_id,
    redirection_data: Box::new(Some(redirection_data)),
    mandate_reference: Box::new(None),
    connector_metadata: None,
    network_txn_id: None,
    connector_response_reference_id: None,
    incremental_authorization_allowed: None,
    charges: None,
};
```

### Handling Conditional Logic

Sometimes connector behavior needs to vary based on configuration or request parameters:

```rust
// In mod.rs

fn get_url(
    &self,
    req: &PaymentsAuthorizeRouterData,
    connectors: &Connectors,
) -> CustomResult<String, errors::ConnectorError> {
    // Example of conditional URL based on request parameters
    if req.request.is_3ds {
        Ok(format!("{}/payments/secure", self.base_url(connectors)))
    } else {
        Ok(format!("{}/payments/standard", self.base_url(connectors)))
    }
}
```

## Best Practices

### 1. Error Handling

Implement comprehensive error handling in your connector:

- Map connector-specific errors to standardized Hyperswitch errors
- Include detailed error messages for debugging
- Log errors with appropriate context
- Handle timeouts and network errors gracefully

```rust
// Example of detailed error handling

match response.status_code {
    401 | 403 => Err(errors::ConnectorError::AuthenticationFailed.into()),
    404 => Err(errors::ConnectorError::ResourceNotFound.into()),
    400..=499 => {
        let error_resp: YourConnectorErrorResponse = response.response.parse_struct("YourConnectorErrorResponse")?;
        Err(errors::ConnectorError::RequestError {
            message: error_resp.message,
            code: error_resp.error,
        }.into())
    },
    500..=599 => Err(errors::ConnectorError::ServerError.into()),
    _ => Err(errors::ConnectorError::UnexpectedResponseError.into()),
}
```

### 2. Logging

Implement appropriate logging in your connector:

- Log connector responses (with sensitive data masked)
- Log errors with context
- Include connection info in logs

```rust
// Example of logging
router_env::logger::info!(connector_request=?request_body, connector="YourConnector");
router_env::logger::info!(connector_response=?response, connector="YourConnector");
```

### 3. Security

Ensure your connector implementation follows security best practices:

- Use `masking::Secret` for sensitive data
- Implement proper authentication
- Validate webhook signatures
- Use HTTPS for all API calls

### 4. Testing

Implement comprehensive tests for your connector:

- Unit tests for transformers
- Integration tests with mock server
- End-to-end tests with sandbox environment

Refer to the [Connector Testing Guide](./connector_testing_guide.md) for details.

## Common Pitfalls

1. **Incorrect Currency Handling**: Ensure you handle the currency unit (major/minor) correctly. Some processors expect amounts in cents/paise, while others expect decimal values.

2. **Incomplete Error Mapping**: Ensure all possible error scenarios from the connector are properly mapped to Hyperswitch error types.

3. **Missing Status Mappings**: Ensure all possible payment status values from the connector are mapped to Hyperswitch status values.

4. **Webhook Verification**: Implement proper webhook signature verification to prevent security issues.

5. **Missing Configuration**: Ensure all required configuration parameters are documented and added to the configuration files.

## Example Connector Template

Refer to the connector template in `connector-template/` directory for a starting point:

```
connector-template/
├── mod.rs
├── transformers.rs
└── test.rs
```

## Next Steps

After implementing your connector:

1. Write comprehensive tests following the [Connector Testing Guide](./connector_testing_guide.md)
2. Configure your connector following the [Connector Configuration Guide](./connector_configuration_guide.md)
3. Submit your connector for review and integration

## See Also

- [Connector Interface Requirements](./connector_interface_guide.md)
- [Connector Testing Guide](./connector_testing_guide.md)
- [Connector Configuration Guide](./connector_configuration_guide.md)
- [Error Handling Guidelines](./error_handling.md)