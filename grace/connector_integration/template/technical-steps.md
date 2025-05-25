# Technical Implementation Steps: {{CONNECTOR_NAME}} Integration

This document provides detailed technical steps for implementing the {{CONNECTOR_NAME}} connector in Hyperswitch. Each step is designed to be atomic, independently compilable, and self-sufficient.

## 1. Authentication Implementation

### Step 1.1: Create Basic Auth Structure
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

use common_utils::errors::CustomResult;
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    connector::utils::RouterData,
    core::errors,
    services::{self, ConnectorIntegration},
    types::{
        self, api, storage::enums,
        transformers::ForeignInto,
    },
};

#[derive(Debug, Clone, Serialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}AuthType {
    pub(super) api_key: Secret<String>,
    // Add additional fields as specified in tech-spec.md Section 3
}
```

### Step 1.2: Implement TryFrom for Auth
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

use crate::connector::utils::ConnectorAuthType;

impl TryFrom<&ConnectorAuthType> for {{CONNECTOR_PASCAL_CASE}}AuthType {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
        if let ConnectorAuthType::HeaderKey { api_key } = auth_type {
            Ok(Self {
                api_key: api_key.to_owned(),
                // Map additional fields
            })
        } else {
            Err(errors::ConnectorError::FailedToObtainAuthType.into())
        }
    }
}
```

### Step 1.3: Create Error Response Structure
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Deserialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}ErrorResponse {
    pub code: Option<String>,
    pub message: Option<String>,
    // Add fields as specified in tech-spec.md Section 4
}
```

## 2. Core Implementation

### Step 2.1: Define Connector Struct
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs

use std::fmt::Debug;

use common_utils::{ext_traits::StringExt, request::RequestContent};
use error_stack::{IntoReport, ResultExt};
use masking::Secret;
use serde::{Deserialize, Serialize};

use crate::{
    configs::settings::Settings,
    connector::utils as connector_utils,
    core::{
        errors::{self, CustomResult},
        payments,
    },
    headers,
    services::{
        self, ConnectorIntegration, ConnectorValidation, PaymentAuthorizeFlow,
        PaymentSyncFlow, RefundExecuteFlow, RefundSyncFlow, PaymentCaptureFlow
    },
    types::{
        self, api, domain, storage::enums, transformers::ForeignInto,
        ErrorResponse, Response, PaymentsAuthorizeData, PaymentsSyncData,
        PaymentsResponseData, RefundsData, RefundsResponseData,
        RefundSyncData,
    },
};

use super::transformers as {{connector_name_lowercase}}_transformers;

#[derive(Debug, Clone)]
pub struct {{CONNECTOR_PASCAL_CASE}};
```

### Step 2.2: Implement Basic ConnectorCommon Methods
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs

use services::ConnectorCommon;
use crate::connector::utils as connector_utils;

impl ConnectorCommon for {{CONNECTOR_PASCAL_CASE}} {
    fn id(&self) -> &'static str {
        "{{connector-name-lowercase}}"
    }

    fn common_get_content_type(&self) -> &'static str {
        "application/json"
    }

    fn base_url<'a>(&self, connectors: &'a Settings) -> &'a str {
        connectors
            .connectors
            .{{connector-name-lowercase}}
            .base_url
            .as_ref()
    }
}
```

### Step 2.3: Implement Auth Header Method
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs

use crate::connector::utils::ConnectorAuthType;
use crate::types::request::Maskable;

impl ConnectorCommon for {{CONNECTOR_PASCAL_CASE}} {
    // Existing methods...

    fn get_auth_header(
        &self,
        auth_type: &ConnectorAuthType,
    ) -> CustomResult<Vec<(String, Maskable<String>)>, errors::ConnectorError> {
        let auth = {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}AuthType::try_from(auth_type)
            .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
        
        Ok(vec![(
            headers::AUTHORIZATION.to_string(),
            format!("Bearer {}", auth.api_key.expose()).into(),
        )])
    }
}
```

### Step 2.4: Implement Error Response Handler
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs

impl ConnectorCommon for {{CONNECTOR_PASCAL_CASE}} {
    // Existing methods...

    fn build_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        let response: {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}ErrorResponse = res
            .response
            .parse_struct("{{CONNECTOR_PASCAL_CASE}}ErrorResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        Ok(ErrorResponse {
            status_code: res.status_code,
            code: response.code,
            message: response.message,
            reason: None,
            attempt_status: None,
            connector_transaction_id: None,
        })
    }
}
```

## 3. Payment Status Implementation

### Step 3.1: Define Payment Status Enum
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum {{CONNECTOR_PASCAL_CASE}}PaymentStatus {
    #[serde(rename = "succeeded")]
    Success,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "pending")]
    Pending,
    // Add additional statuses as specified in tech-spec.md Section 6.X.2
}
```

### Step 3.2: Implement Status Conversion
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

use crate::types::storage::enums as storage_enums;

impl From<{{CONNECTOR_PASCAL_CASE}}PaymentStatus> for storage_enums::AttemptStatus {
    fn from(status: {{CONNECTOR_PASCAL_CASE}}PaymentStatus) -> Self {
        match status {
            {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Success => Self::Charged,
            {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Failed => Self::Failure,
            {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Pending => Self::Pending,
            // Map additional statuses
        }
    }
}
```

## 4. Authorize Flow Implementation

### Step 4.1: Define Card Data Structure (if needed)
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Serialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}CardData {
    pub number: masking::Secret<String>,
    pub expiry_month: Secret<String>,
    pub expiry_year: Secret<String>,
    pub cvc: Secret<String>,
    // Additional fields as needed
}
```

### Step 4.2: Define Authorize Request Structure
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Serialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}AuthorizeRequest {
    pub amount: i64,
    pub currency: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payment_method: Option<{{CONNECTOR_PASCAL_CASE}}CardData>,
    pub description: Option<String>,
    pub metadata: Option<serde_json::Value>,
    // Additional fields as specified in tech-spec.md Section 6.X.1
}
```

### Step 4.3: Implement RouterData for {{CONNECTOR_PASCAL_CASE}}
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

pub struct {{CONNECTOR_PASCAL_CASE}}RouterData<T> {
    pub amount: i64,
    pub router_data: T,
}

impl<T> TryFrom<&types::RouterData<T, types::PaymentsAuthorizeData>> 
    for {{CONNECTOR_PASCAL_CASE}}RouterData<T>
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &types::RouterData<T, types::PaymentsAuthorizeData>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: item.amount,
            router_data: item.router_data.clone(),
        })
    }
}
```

### Step 4.4: Implement TryFrom for Authorize Request
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

impl TryFrom<&{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsAuthorizeData>> 
    for {{CONNECTOR_PASCAL_CASE}}AuthorizeRequest 
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsAuthorizeData>) -> Result<Self, Self::Error> {
        let auth_data = item.router_data;
        let currency = auth_data.request.currency.to_string();
        
        let payment_method = auth_data.request.payment_method_data.clone().map(|pmd| {
            match pmd {
                domain::PaymentMethodData::Card(card) => {{CONNECTOR_PASCAL_CASE}}CardData {
                    number: card.card_number.clone(),
                    expiry_month: card.card_exp_month.clone(),
                    expiry_year: card.card_exp_year.clone(),
                    cvc: card.card_cvc.clone(),
                },
                _ => Err(errors::ConnectorError::NotImplemented("Payment method".to_string()).into()),
            }
        }).transpose()?;

        Ok(Self {
            amount: item.amount,
            currency,
            payment_method,
            description: auth_data.description.clone(),
            metadata: None,
            // Map additional fields
        })
    }
}
```

### Step 4.5: Define Authorize Response Structure
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Deserialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}AuthorizeResponse {
    pub id: String,
    pub status: {{CONNECTOR_PASCAL_CASE}}PaymentStatus,
    pub amount: i64,
    pub currency: String,
    // Additional fields as specified in tech-spec.md Section 6.X.2
}
```

### Step 4.6: Implement TryFrom for Authorize Response
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

use crate::types::{
    ResponseRouterData,
    api::PaymentsAuthorizeRouterData,
    storage::enums::ConnectorStatus,
};

impl TryFrom<ResponseRouterData<api::Authorize, {{CONNECTOR_PASCAL_CASE}}AuthorizeResponse, PaymentsAuthorizeData, PaymentsResponseData>> 
    for PaymentsAuthorizeRouterData 
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<api::Authorize, {{CONNECTOR_PASCAL_CASE}}AuthorizeResponse, PaymentsAuthorizeData, PaymentsResponseData>
    ) -> Result<Self, Self::Error> {
        let (status, error_response) = match item.response.status {
            {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Success => (Ok(storage_enums::AttemptStatus::Charged), None),
            {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Failed => (
                Ok(storage_enums::AttemptStatus::Failure),
                Some(ErrorResponse {
                    code: None,
                    message: Some("Payment Failed".to_string()),
                    reason: None,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.id.clone()),
                }),
            ),
            {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Pending => (Ok(storage_enums::AttemptStatus::Pending), None),
            // Handle additional statuses
        };

        Ok(Self {
            status,
            response: error_response.map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?,
            payment_response_data: PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            },
        })
    }
}
```

### Step 4.7: Implement ConnectorIntegration for Authorize
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs

impl ConnectorIntegration<api::Authorize, PaymentsAuthorizeData, PaymentsResponseData> for {{CONNECTOR_PASCAL_CASE}} {
    fn get_headers(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &Settings,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        _req: &types::PaymentsAuthorizeRouterData,
        connectors: &Settings,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!("{}/payments", self.base_url(connectors)))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsAuthorizeRouterData,
        connectors: &Settings,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let router_data = {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}RouterData::try_from(req)?;
        let connector_req = {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}AuthorizeRequest::try_from(&router_data)?;
        
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsAuthorizeRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsAuthorizeRouterData, errors::ConnectorError> {
        let response: {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}AuthorizeResponse = res
            .response
            .parse_struct("{{CONNECTOR_PASCAL_CASE}}AuthorizeResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}
```

## 5. Registration and Configuration

### Step 5.1: Register in Connector Enum
```rust
// In crates/common_enums/src/connector_enums.rs

#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "camelCase")]
pub enum Connector {
    // ... existing connectors
    {{CONNECTOR_PASCAL_CASE}},
}

// Update impl From<Connector> for &'static str
impl From<Connector> for &'static str {
    fn from(connector: Connector) -> Self {
        match connector {
            // ... existing matches
            Connector::{{CONNECTOR_PASCAL_CASE}} => "{{connector-name-lowercase}}",
        }
    }
}
```

### Step 5.2: Register in RoutableConnectors (if applicable)
```rust
// In crates/common_enums/src/connector_enums.rs

#[derive(Debug, Clone, Hash, PartialEq, Eq, Copy, Serialize, Deserialize, EnumString, Display)]
#[strum(serialize_all = "snake_case")]
pub enum RoutableConnectors {
    // ... existing connectors
    {{CONNECTOR_PASCAL_CASE}},
}

// Update impl TryFrom<&str> for RoutableConnectors
impl TryFrom<&str> for RoutableConnectors {
    type Error = errors::ParsingError;
    fn try_from(s: &str) -> Result<Self, Self::Error> {
        match s {
            // ... existing matches
            "{{connector-name-lowercase}}" => Ok(Self::{{CONNECTOR_PASCAL_CASE}}),
            _ => Err(errors::ParsingError::UnknownVariant {
                message: format!("Unknown connector: {s}"),
            }),
        }
    }
}
```

### Step 5.3: Add Development Configuration
```toml
# In crates/connector_configs/toml/development.toml

[connectors.{{connector-name-lowercase}}]
base_url = "{{connector-base-url}}"
```

### Step 5.4: Add Test Authentication
```toml
# In crates/router/tests/connectors/sample_auth.toml

[{{connector-name-lowercase}}]
api_key = "test_api_key"
# Additional auth details as needed
```

## 6. Testing Authorize Flow

### Step 6.1: Implement Basic Test Setup
```rust
// In crates/router/tests/connectors/{{connector-name-lowercase}}.rs

use std::str::FromStr;

use masking::Secret;
use router::types::{self, domain::Email, storage::enums};

use crate::connector::utils as test_utils;
use crate::connector::utils::ConnectorAuthType;

#[derive(Clone, Copy)]
struct {{CONNECTOR_PASCAL_CASE}}Test;

impl test_utils::ConnectorActions for {{CONNECTOR_PASCAL_CASE}}Test {}
impl test_utils::PaymentAuthorizeType for {{CONNECTOR_PASCAL_CASE}}Test {}

impl test_utils::Connector for {{CONNECTOR_PASCAL_CASE}}Test {
    fn get_data(&self) -> types::api::ConnectorData {
        types::api::ConnectorData {
            connector: types::storage::enums::Connector::{{CONNECTOR_PASCAL_CASE}},
            connector_name: types::storage::enums::Connector::{{CONNECTOR_PASCAL_CASE}}.to_string(),
            get_token: types::storage::enums::ConnectorTokenKind::HeaderKey,
        }
    }

    fn get_auth_token(&self) -> ConnectorAuthType {
        ConnectorAuthType::HeaderKey {
            api_key: Secret::new("test_api_key".to_string()),
        }
    }

    fn get_name(&self) -> String {
        String::from("{{connector-name-lowercase}}")
    }
}
```

### Step 6.2: Implement Payment Data Setup
```rust
// In crates/router/tests/connectors/{{connector-name-lowercase}}.rs

fn get_default_payment_info() -> Option<types::PaymentsAuthorizeData> {
    Some(types::PaymentsAuthorizeData {
        payment_method_data: types::api::PaymentMethodData::Card(types::api::Card {
            card_number: Secret::new("4242424242424242".to_string()),
            card_exp_month: Secret::new("10".to_string()),
            card_exp_year: Secret::new("2025".to_string()),
            card_cvc: Secret::new("123".to_string()),
            card_holder_name: Secret::new("John Doe".to_string()),
            card_issuer: None,
            card_network: None,
            card_type: None,
            card_issuing_country: None,
            bank_code: None,
            nick_name: Some(Secret::new("test card".to_string())),
        }),
        amount: 100,
        currency: enums::Currency::USD,
        statement_descriptor_name: None,
        statement_descriptor_suffix: None,
        capture_method: Some(enums::CaptureMethod::Automatic),
        router_return_url: Some("https://example.com/return".to_string()),
        webhook_url: Some("https://example.com/webhook".to_string()),
        billing: None,
        shipping: None,
        email: None,
        payment_experience: None,
        customer_id: None,
        setup_future_usage: None,
        mandate_id: None,
        setup_mandate_details: None,
        browser_info: None,
        order_details: None,
        order_category: None,
        session_token: None,
        enrolled_for_3ds: false,
        related_transaction_id: None,
        payment_method_type: None,
        payment_method_id: None,
        authentication_data: None,
        connector_meta_data: None,
        customer_acceptance: None,
        surcharge_details: None,
        request_incremental_authorization: false,
        metadata: None,
        feature_metadata: None,
        payout_constraints: None,
        decoupled_authentication: None,
        mandate_data: None,
        recurring_details: None,
        return_url: None,
        individual: None,
        post_authentication: None,
        previous_attempt_id: None,
        client_secret: None,
        merchant_connector_id: None,
        device: None,
        description: None,
        loyalty_points: None,
        payment_method_billing: None,
        payment_token: None,
        reference_id: None,
        risk_score: None,
        offer_details: None,
        duplicate_check_criteria: None,
        incremental_authorization_details: None,
        payment_method_token: None,
        payment_service_data: None,
        payment_value_data: None,
        connector_reference_id: None,
        additional_details: None,
        industry_details: None,
        debtor_details: None,
        creditor_details: None,
    })
}
```

### Step 6.3: Implement Authorize Test
```rust
// In crates/router/tests/connectors/{{connector-name-lowercase}}.rs

#[test]
fn test_{{connector-name-lowercase}}_authorize_success() {
    let connector = {{CONNECTOR_PASCAL_CASE}}Test;
    let payment_info = get_default_payment_info().unwrap();
    
    let test_response = connector
        .authorize_payment(payment_info, None)
        .await
        .expect("Authorize payment failed");
    
    assert_eq!(test_response.status, enums::AttemptStatus::Charged);
}
```

## 7. Capture Flow Implementation

### Step 7.1: Define Capture Request Structure
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Serialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}CaptureRequest {
    pub amount: Option<i64>,
    // Additional fields as specified in tech-spec.md Section 6.Y.1
}

impl TryFrom<&{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsCaptureData>> for {{CONNECTOR_PASCAL_CASE}}CaptureRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsCaptureData>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: Some(item.amount),
            // Map additional fields
        })
    }
}
```

### Step 7.2: Define Capture Response Structure
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Deserialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}CaptureResponse {
    pub id: String,
    pub status: {{CONNECTOR_PASCAL_CASE}}PaymentStatus,
    pub amount: i64,
    pub currency: String,
    // Additional fields as specified in tech-spec.md Section 6.Y.2
}
```

### Step 7.3: Implement TryFrom for Capture Response
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

use crate::types::{
    ResponseRouterData,
    api::{PaymentsCaptureRouterData, Capture},
};

impl TryFrom<ResponseRouterData<Capture, {{CONNECTOR_PASCAL_CASE}}CaptureResponse, PaymentsCaptureData, PaymentsCaptureResponseData>> 
    for PaymentsCaptureRouterData 
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<Capture, {{CONNECTOR_PASCAL_CASE}}CaptureResponse, PaymentsCaptureData, PaymentsCaptureResponseData>
    ) -> Result<Self, Self::Error> {
        let (status, error_response) = match item.response.status {
            {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Success => (Ok(storage_enums::AttemptStatus::Charged), None),
            {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Failed => (
                Ok(storage_enums::AttemptStatus::Failure),
                Some(ErrorResponse {
                    code: None,
                    message: Some("Capture Failed".to_string()),
                    reason: None,
                    status_code: item.http_code,
                    attempt_status: None,
                    connector_transaction_id: Some(item.response.id.clone()),
                }),
            ),
            {{CONNECTOR_PASCAL_CASE}}PaymentStatus::Pending => (Ok(storage_enums::AttemptStatus::Pending), None),
            // Handle additional statuses
        };

        Ok(Self {
            status,
            response: error_response.map_err(|_| errors::ConnectorError::ResponseHandlingFailed)?,
            payment_response_data: PaymentsCaptureResponseData {
                connector_transaction_id: item.response.id,
                connector_metadata: None,
                network_txn_id: None,
            },
        })
    }
}
```

### Step 7.4: Implement ConnectorIntegration for Capture
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs

impl ConnectorIntegration<api::Capture, PaymentsCaptureData, PaymentsCaptureResponseData> for {{CONNECTOR_PASCAL_CASE}} {
    fn get_headers(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &Settings,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &Settings,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/payments/{}/capture",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::PaymentsCaptureRouterData,
        connectors: &Settings,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let router_data = {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}RouterData::try_from(req)?;
        let connector_req = {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}CaptureRequest::try_from(&router_data)?;
        
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsCaptureRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsCaptureRouterData, errors::ConnectorError> {
        let response: {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}CaptureResponse = res
            .response
            .parse_struct("{{CONNECTOR_PASCAL_CASE}}CaptureResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}
```

## 8. Payment Sync Flow Implementation

### Step 8.1: Define Sync Request Structure (if needed)
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Serialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}PsyncRequest {
    // Request fields as specified in tech-spec.md Section 6.Z.1
}

impl TryFrom<&{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsSyncData>> for {{CONNECTOR_PASCAL_CASE}}PsyncRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsSyncData>) -> Result<Self, Self::Error> {
        // Implementation as specified in tech-spec.md Section 6.Z.1
        Ok(Self {
            // Map fields
        })
    }
}
```

### Step 8.2: Define Sync Response Structure
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Deserialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}PsyncResponse {
    pub id: String,
    pub status: {{CONNECTOR_PASCAL_CASE}}PaymentStatus,
    // Additional fields as specified in tech-spec.md Section 6.Z.2
}
```

### Step 8.3: Implement TryFrom for Sync Response
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

use crate::types::{
    ResponseRouterData,
    api::{PaymentsSyncRouterData, PSync},
};

impl TryFrom<ResponseRouterData<PSync, {{CONNECTOR_PASCAL_CASE}}PsyncResponse, PaymentsSyncData, PaymentsSyncResponseData>> 
    for PaymentsSyncRouterData 
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<PSync, {{CONNECTOR_PASCAL_CASE}}PsyncResponse, PaymentsSyncData, PaymentsSyncResponseData>
    ) -> Result<Self, Self::Error> {
        let status = storage_enums::AttemptStatus::from(item.response.status);
        
        Ok(Self {
            status: Ok(status),
            response: Ok(types::PaymentsResponseData::TransactionResponse {
                resource_id: types::ResponseId::ConnectorTransactionId(item.response.id),
                redirection_data: None,
                redirect: false,
                mandate_reference: None,
                connector_metadata: None,
                network_txn_id: None,
            }),
            ..item.data.clone()
        })
    }
}
```

### Step 8.4: Implement ConnectorIntegration for PSync
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs

impl ConnectorIntegration<api::PSync, PaymentsSyncData, PaymentsSyncResponseData> for {{CONNECTOR_PASCAL_CASE}} {
    fn get_headers(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &Settings,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::PaymentsSyncRouterData,
        connectors: &Settings,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/payments/{}",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn handle_response(
        &self,
        data: &types::PaymentsSyncRouterData,
        res: Response,
    ) -> CustomResult<types::PaymentsSyncRouterData, errors::ConnectorError> {
        let response: {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}PsyncResponse = res
            .response
            .parse_struct("{{CONNECTOR_PASCAL_CASE}}PsyncResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}
```

## 9. Refund Flow Implementation

### Step 9.1: Define Refund Request Structure
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Serialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}RefundRequest {
    pub amount: Option<i64>,
    pub reason: Option<String>,
    // Additional fields as specified in tech-spec.md Section 6.W.1
}

impl TryFrom<&{{CONNECTOR_PASCAL_CASE}}RouterData<&RefundsData>> for {{CONNECTOR_PASCAL_CASE}}RefundRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(item: &{{CONNECTOR_PASCAL_CASE}}RouterData<&RefundsData>) -> Result<Self, Self::Error> {
        Ok(Self {
            amount: Some(item.amount),
            reason: item.router_data.refund_reason.clone(),
            // Map additional fields
        })
    }
}
```

### Step 9.2: Define Refund Response Structure
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

#[derive(Debug, Deserialize)]
pub struct {{CONNECTOR_PASCAL_CASE}}RefundResponse {
    pub id: String,
    pub status: String,
    pub amount: i64,
    // Additional fields as specified in tech-spec.md Section 6.W.2
}
```

### Step 9.3: Implement TryFrom for Refund Response
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs

use crate::types::{
    ResponseRouterData,
    api::{RefundsRouterData, Execute},
    storage::enums::RefundStatus,
};

impl TryFrom<ResponseRouterData<Execute, {{CONNECTOR_PASCAL_CASE}}RefundResponse, RefundsData, RefundsResponseData>> 
    for RefundsRouterData<RefundsResponseData> 
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: ResponseRouterData<Execute, {{CONNECTOR_PASCAL_CASE}}RefundResponse, RefundsData, RefundsResponseData>
    ) -> Result<Self, Self::Error> {
        let refund_status = match item.response.status.as_str() {
            "succeeded" => RefundStatus::Success,
            "failed" => RefundStatus::Failure,
            "pending" => RefundStatus::Pending,
            _ => RefundStatus::Pending,
        };
        
        Ok(Self {
            response: Ok(RefundsResponseData {
                connector_refund_id: item.response.id,
                refund_status,
            }),
            ..item.data.clone()
        })
    }
}
```

### Step 9.4: Implement ConnectorIntegration for Refund
```rust
// In crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs

impl ConnectorIntegration<api::Execute, RefundsData, RefundsResponseData> for {{CONNECTOR_PASCAL_CASE}} {
    fn get_headers(
        &self,
        req: &types::RefundsRouterData<RefundsData>,
        connectors: &Settings,
    ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
        self.get_auth_header(&req.connector_auth_type)
    }

    fn get_content_type(&self) -> &'static str {
        self.common_get_content_type()
    }

    fn get_url(
        &self,
        req: &types::RefundsRouterData<RefundsData>,
        connectors: &Settings,
    ) -> CustomResult<String, errors::ConnectorError> {
        Ok(format!(
            "{}/payments/{}/refunds",
            self.base_url(connectors),
            req.request.connector_transaction_id
        ))
    }

    fn get_request_body(
        &self,
        req: &types::RefundsRouterData<RefundsData>,
        connectors: &Settings,
    ) -> CustomResult<RequestContent, errors::ConnectorError> {
        let router_data = {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}RouterData::try_from(req)?;
        let connector_req = {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}RefundRequest::try_from(&router_data)?;
        
        Ok(RequestContent::Json(Box::new(connector_req)))
    }

    fn handle_response(
        &self,
        data: &types::RefundsRouterData<RefundsData>,
        res: Response,
    ) -> CustomResult<types::RefundsRouterData<RefundsResponseData>, errors::ConnectorError> {
        let response: {{connector_name_lowercase}}_transformers::{{CONNECTOR_PASCAL_CASE}}RefundResponse = res
            .response
            .parse_struct("{{CONNECTOR_PASCAL_CASE}}RefundResponse")
            .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;

        types::ResponseRouterData {
            response,
            data: data.clone(),
            http_code: res.status_code,
        }
        .try_into()
        .change_context(errors::ConnectorError::ResponseHandlingFailed)
    }

    fn get_error_response(
        &self,
        res: Response,
    ) -> CustomResult<ErrorResponse, errors::ConnectorError> {
        self.build_error_response(res)
    }
}
```
