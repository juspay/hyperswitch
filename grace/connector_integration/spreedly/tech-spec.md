# Spreedly Technical Specification

## 1. Connector Overview
- **Connector Name**: Spreedly
- **Connector PascalCase Name**: Spreedly
- **Connector Lowercase Name**: spreedly
- **API Documentation URL**: [Link to Spreedly API Docs](https://developer.spreedly.com)
- **Core Purpose**: Spreedly is a global payments orchestration platform that enables businesses to connect with multiple payment gateways and services through a single API. It provides features like secure card vaulting, tokenization, and support for various payment methods, facilitating streamlined and secure payment processing.
- **Supported Payment Methods by Hyperswitch Integration**: Cards (VISA, Mastercard, etc.)
- **Key Workflows to be Implemented**: Authorize, Capture, Refund, Payment Sync

## 2. Integration Project Structure
- **Main Logic File**: `crates/hyperswitch_connectors/src/connectors/spreedly.rs`
- **Transformers File**: `crates/hyperswitch_connectors/src/connectors/spreedly/transformers.rs`
- **Test File**: `crates/router/tests/connectors/spreedly.rs`
- **Enum Registrations**: `crates/common_enums/src/connector_enums.rs`
- **Backend Configuration**: `crates/connector_configs/toml/development.toml`
- **Test Authentication**: `crates/router/tests/connectors/sample_auth.toml`
- **(Optional) Control Center UI Files**:
    - `hyperswitch-control-center/src/screens/HyperSwitch/Connectors/ConnectorTypes.res`
    - `hyperswitch-control-center/src/screens/HyperSwitch/Connectors/ConnectorUtils.res`
    - `hyperswitch-control-center/public/hyperswitch/Gateway/SPREEDLY.SVG`

## 3. Authentication Mechanism
- **Authentication Type**: HTTP Basic Authentication over HTTPS
- **Credentials Required**: `environment_key` (acts as username), `access_secret` (acts as password)
- **`SpreedlyAuthType` Struct Definition (`transformers.rs`):**
  ```rust
  #[derive(Debug, Serialize)]
  pub struct SpreedlyAuthType {
      pub environment_key: String,
      pub access_secret: String,
  }
  
  impl TryFrom<&ConnectorAuthType> for SpreedlyAuthType {
      type Error = error_stack::Report<errors::ConnectorError>;
      fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
          if let ConnectorAuthType::HeaderKey {
              api_key,
              key1,
          } = auth_type {
              Ok(Self {
                  environment_key: api_key.to_string(),
                  access_secret: key1.to_string(),
              })
          } else {
              Err(errors::ConnectorError::FailedToObtainAuthType.into())
          }
      }
  }
  ```
- **`get_auth_header()` Implementation Sketch (`spreedly.rs`):**
  ```rust
  fn get_auth_header(
      &self,
      auth_type: &ConnectorAuthType,
  ) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
      let auth = SpreedlyAuthType::try_from(auth_type)
          .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
      
      let auth_string = format!("{}:{}", auth.environment_key, auth.access_secret);
      let encoded_auth = consts::BASE64_ENGINE.encode(auth_string);
      
      Ok(vec![(
          headers::AUTHORIZATION.to_string(),
          format!("Basic {}", encoded_auth).into(),
      )])
  }
  ```

## 4. Error Handling
- **Connector Error Response Structure (`transformers.rs`):**
  ```rust
  #[derive(Debug, Deserialize)]
  pub struct SpreedlyErrorResponse {
      pub errors: Option<Vec<SpreedlyError>>,
  }
  
  #[derive(Debug, Deserialize)]
  pub struct SpreedlyError {
      pub key: Option<String>,
      pub message: Option<String>,
  }
  ```
- **`build_error_response()` Implementation Sketch (`spreedly.rs`):**
  ```rust
  fn build_error_response(
      &self,
      res: types::Response,
      event_builder: Option<&mut ConnectorEvent>,
  ) -> CustomResult<types::ErrorResponse, errors::ConnectorError> {
      let response: SpreedlyErrorResponse = res
          .response
          .parse_struct("SpreedlyErrorResponse")
          .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
      
      let mut reasons = vec![];
      if let Some(errors) = response.errors {
          for error in errors {
              if let Some(message) = error.message {
                  reasons.push(message);
              }
          }
      }
      
      let message = reasons.join(" ");
      
      Ok(types::ErrorResponse {
          status_code: res.status_code,
          code: res.status_code.to_string(),
          message,
          reason: None,
          attempt_status: None,
          connector_transaction_id: None,
      })
  }
  ```
- **Key Error Code Mappings**:
  - 401 Unauthorized: Authentication failed
  - 422 Unprocessable Entity: Validation errors

## 5. Common Connector Details (`ConnectorCommon` in `spreedly.rs`)
- **`id()`**: `"spreedly"`
- **`get_currency_unit()`**: `api::CurrencyUnit::Minor` (Spreedly uses minor units like cents)
- **`common_get_content_type()`**: `"application/json"`
- **`base_url()`**: `connectors.spreedly.base_url` (Will be set to "https://core.spreedly.com/v1")

## 6. Feature Specification: Payment Flows

### 6.1 Flow: Authorize for Cards
- **Hyperswitch Trait**: `ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>`
- **Connector API Endpoint(s)**:
  - Method: POST
  - URL Path: `/v1/gateways/{gateway_token}/authorize.json`
- **Amount Handling**: Uses minor units (cents) - no conversion needed

#### 6.1.1 Request Transformation (`transformers.rs`)
- **`SpreedlyPaymentsRequest` Struct Definition**:
  ```rust
  #[derive(Debug, Serialize)]
  pub struct SpreedlyPaymentsRequest {
      pub transaction: SpreedlyTransactionRequest,
  }
  
  #[derive(Debug, Serialize)]
  pub struct SpreedlyTransactionRequest {
      pub credit_card: SpreedlyCardDetails,
      pub amount: i64,
      pub currency_code: String,
  }
  
  #[derive(Debug, Serialize)]
  pub struct SpreedlyCardDetails {
      pub number: cards::CardNumber,
      #[serde(rename = "verification_value")]
      pub cvv: cards::CVV,
      pub month: String,
      pub year: String,
      #[serde(rename = "first_name")]
      pub first_name: Option<String>,
      #[serde(rename = "last_name")]
      pub last_name: Option<String>,
  }
  ```
- **`TryFrom<&SpreedlyRouterData<&PaymentsAuthorizeData>> for SpreedlyPaymentsRequest` Implementation**:
  ```rust
  impl TryFrom<&SpreedlyRouterData<&PaymentsAuthorizeData>> for SpreedlyPaymentsRequest {
      type Error = error_stack::Report<errors::ConnectorError>;
      fn try_from(item: &SpreedlyRouterData<&PaymentsAuthorizeData>) -> Result<Self, Self::Error> {
          match item.request.payment_method_data {
              api::PaymentMethodData::Card(ref card) => {
                  let exp_month = card.card_exp_month.clone().ok_or(
                      errors::ConnectorError::MissingRequiredField {
                          field_name: "card_exp_month",
                      },
                  )?;
                  let exp_year = card.card_exp_year.clone().ok_or(
                      errors::ConnectorError::MissingRequiredField {
                          field_name: "card_exp_year",
                      },
                  )?;
                  
                  let card_details = SpreedlyCardDetails {
                      number: card.card_number.clone(),
                      cvv: card.card_cvc.clone(),
                      month: exp_month,
                      year: exp_year,
                      first_name: item.request.get_billing_first_name()?,
                      last_name: item.request.get_billing_last_name()?,
                  };
                  
                  Ok(Self {
                      transaction: SpreedlyTransactionRequest {
                          credit_card: card_details,
                          amount: item.amount,
                          currency_code: item.request.currency.to_string(),
                      },
                  })
              }
              _ => Err(errors::ConnectorError::NotImplemented("Payment methods other than card".to_string()).into()),
          }
      }
  }
  ```

#### 6.1.2 Response Transformation (`transformers.rs`)
- **`SpreedlyPaymentStatus` Enum Definition**:
  ```rust
  #[derive(Debug, Deserialize, Serialize)]
  #[serde(rename_all = "snake_case")]
  pub enum SpreedlyPaymentStatus {
      Succeeded,
      Failed,
      #[serde(other)]
      Unknown,
  }
  
  impl From<SpreedlyPaymentStatus> for common_enums::AttemptStatus {
      fn from(status: SpreedlyPaymentStatus) -> Self {
          match status {
              SpreedlyPaymentStatus::Succeeded => Self::Charged,
              SpreedlyPaymentStatus::Failed => Self::Failure,
              SpreedlyPaymentStatus::Unknown => Self::Pending,
          }
      }
  }
  ```
- **`SpreedlyPaymentsResponse` Struct Definition**:
  ```rust
  #[derive(Debug, Deserialize)]
  pub struct SpreedlyPaymentsResponse {
      pub transaction: SpreedlyTransactionResponse,
  }
  
  #[derive(Debug, Deserialize)]
  pub struct SpreedlyTransactionResponse {
      pub token: String,
      pub succeeded: bool,
      #[serde(rename = "transaction_type")]
      pub transaction_type: String,
      pub amount: i64,
      pub payment_method: Option<SpreedlyPaymentMethod>,
  }
  
  #[derive(Debug, Deserialize)]
  pub struct SpreedlyPaymentMethod {
      pub token: String,
  }
  ```
- **`TryFrom<ResponseRouterData<Authorize, SpreedlyPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>> for PaymentsAuthorizeRouterData` Implementation**:
  ```rust
  impl TryFrom<ResponseRouterData<Authorize, SpreedlyPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>> for PaymentsAuthorizeRouterData {
      type Error = error_stack::Report<errors::ConnectorError>;
      fn try_from(item: ResponseRouterData<Authorize, SpreedlyPaymentsResponse, PaymentsAuthorizeData, PaymentsResponseData>) -> Result<Self, Self::Error> {
          let status = if item.response.transaction.succeeded {
              common_enums::AttemptStatus::Charged
          } else {
              common_enums::AttemptStatus::Failure
          };
          
          Ok(Self {
              status,
              response: item.info.clone(),
              amount_received: item.response.transaction.amount,
              router_data: item.data,
              connector_transaction_id: Some(item.response.transaction.token),
              payment_method_token: item.response.transaction.payment_method.map(|method| method.token),
              ..Default::default()
          })
      }
  }
  ```

#### 6.1.3 Main Logic (`spreedly.rs`)
- **`get_url()` Implementation**:
  ```rust
  fn get_url(
      &self,
      req: &PaymentsAuthorizeData,
      connectors: &Connectors,
  ) -> CustomResult<String, errors::ConnectorError> {
      let connector_auth = SpreedlyAuthType::try_from(&req.connector_auth_type)
          .change_context(errors::ConnectorError::FailedToObtainAuthType)?;
      let gateway_token = connector_auth.environment_key;
      
      Ok(format!("{}/gateways/{}/authorize.json", self.base_url(connectors), gateway_token))
  }
  ```
- **`get_request_body()` Implementation**:
  ```rust
  fn get_request_body(
      &self,
      req: &PaymentsAuthorizeData,
      connectors: &Connectors,
  ) -> CustomResult<RequestContent, errors::ConnectorError> {
      let router_data = SpreedlyRouterData {
          amount: req.request.amount,
          router_data: req,
      };
      
      let spreedly_req = SpreedlyPaymentsRequest::try_from(&router_data)?;
      Ok(RequestContent::Json(Box::new(spreedly_req)))
  }
  ```
- **`handle_response()` Implementation**:
  ```rust
  fn handle_response(
      &self,
      data: &PaymentsAuthorizeData,
      event_builder: Option<&mut ConnectorEvent>,
      res: types::Response,
  ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
      let response: SpreedlyPaymentsResponse = res
          .response
          .parse_struct("SpreedlyPaymentsResponse")
          .change_context(errors::ConnectorError::ResponseDeserializationFailed)?;
      
      Ok(ResponseRouterData {
          response,
          data: data.clone(),
          http_code: res.status_code,
          info: PaymentsResponseData::empty(),
      }
      .try_into()?)
  }
  ```

### 6.2 Flow: Capture
- **Hyperswitch Trait**: `ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsResponseData>`
- **Connector API Endpoint(s)**:
  - Method: POST
  - URL Path: `/v1/transactions/{transaction_token}/capture.json`
- **Amount Handling**: Uses minor units (cents) - no conversion needed

#### 6.2.1 Request Transformation (`transformers.rs`)
- **`SpreedlyCaptureRequest` Struct Definition**:
  ```rust
  #[derive(Debug, Serialize)]
  pub struct SpreedlyCaptureRequest {
      pub transaction: SpreedlyCaptureTransactionRequest,
  }
  
  #[derive(Debug, Serialize)]
  pub struct SpreedlyCaptureTransactionRequest {
      pub amount: i64,
  }
  ```
- **`TryFrom<&SpreedlyRouterData<&PaymentsCaptureData>> for SpreedlyCaptureRequest` Implementation**:
  ```rust
  impl TryFrom<&SpreedlyRouterData<&PaymentsCaptureData>> for SpreedlyCaptureRequest {
      type Error = error_stack::Report<errors::ConnectorError>;
      fn try_from(item: &SpreedlyRouterData<&PaymentsCaptureData>) -> Result<Self, Self::Error> {
          Ok(Self {
              transaction: SpreedlyCaptureTransactionRequest {
                  amount: item.amount,
              },
          })
      }
  }
  ```

#### 6.2.2 Response Transformation (`transformers.rs`)
- **`SpreedlyCaptureResponse` Struct Definition**:
  ```rust
  #[derive(Debug, Deserialize)]
  pub struct SpreedlyCaptureResponse {
      pub transaction: SpreedlyCaptureTransactionResponse,
  }
  
  #[derive(Debug, Deserialize)]
  pub struct SpreedlyCaptureTransactionResponse {
      pub token: String,
      pub succeeded: bool,
      #[serde(rename = "transaction_type")]
      pub transaction_type: String,
      pub amount: i64,
  }
  ```
- **`TryFrom<ResponseRouterData<Capture, SpreedlyCaptureResponse, PaymentsCaptureData, PaymentsResponseData>> for PaymentsCaptureRouterData` Implementation**:
  ```rust
  impl TryFrom<ResponseRouterData<Capture, SpreedlyCaptureResponse, PaymentsCaptureData, PaymentsResponseData>> for PaymentsCaptureRouterData {
      type Error = error_stack::Report<errors::ConnectorError>;
      fn try_from(item: ResponseRouterData<Capture, SpreedlyCaptureResponse, PaymentsCaptureData, PaymentsResponseData>) -> Result<Self, Self::Error> {
          let status = if item.response.transaction.succeeded {
              common_enums::AttemptStatus::Charged
          } else {
              common_enums::AttemptStatus::Failure
          };
          
          Ok(Self {
              status,
              response: item.info.clone(),
              amount_received: item.response.transaction.amount,
              router_data: item.data,
              connector_transaction_id: Some(item.response.transaction.token),
              ..Default::default()
          })
      }
  }
  ```

### 6.3 Flow: Refund
- **Hyperswitch Trait**: `ConnectorIntegration<Execute, RefundsData, RefundsResponseData>`
- **Connector API Endpoint(s)**:
  - Method: POST
  - URL Path: `/v1/transactions/{transaction_token}/credit.json`
- **Amount Handling**: Uses minor units (cents) - no conversion needed

#### 6.3.1 Request Transformation (`transformers.rs`)
- **`SpreedlyRefundRequest` Struct Definition**:
  ```rust
  #[derive(Debug, Serialize)]
  pub struct SpreedlyRefundRequest {
      pub transaction: SpreedlyRefundTransactionRequest,
  }
  
  #[derive(Debug, Serialize)]
  pub struct SpreedlyRefundTransactionRequest {
      pub amount: i64,
  }
  ```
- **`TryFrom<&SpreedlyRouterData<&RefundsData>> for SpreedlyRefundRequest` Implementation**:
  ```rust
  impl TryFrom<&SpreedlyRouterData<&RefundsData>> for SpreedlyRefundRequest {
      type Error = error_stack::Report<errors::ConnectorError>;
      fn try_from(item: &SpreedlyRouterData<&RefundsData>) -> Result<Self, Self::Error> {
          Ok(Self {
              transaction: SpreedlyRefundTransactionRequest {
                  amount: item.amount,
              },
          })
      }
  }
  ```

#### 6.3.2 Response Transformation (`transformers.rs`)
- **`SpreedlyRefundStatus` Enum Definition**:
  ```rust
  #[derive(Debug, Deserialize, Serialize)]
  #[serde(rename_all = "snake_case")]
  pub enum SpreedlyRefundStatus {
      Succeeded,
      Failed,
      #[serde(other)]
      Unknown,
  }
  
  impl From<SpreedlyRefundStatus> for common_enums::RefundStatus {
      fn from(status: SpreedlyRefundStatus) -> Self {
          match status {
              SpreedlyRefundStatus::Succeeded => Self::Success,
              SpreedlyRefundStatus::Failed => Self::Failure,
              SpreedlyRefundStatus::Unknown => Self::Pending,
          }
      }
  }
  ```
- **`SpreedlyRefundResponse` Struct Definition**:
  ```rust
  #[derive(Debug, Deserialize)]
  pub struct SpreedlyRefundResponse {
      pub transaction: SpreedlyRefundTransactionResponse,
  }
  
  #[derive(Debug, Deserialize)]
  pub struct SpreedlyRefundTransactionResponse {
      pub token: String,
      pub succeeded: bool,
      #[serde(rename = "transaction_type")]
      pub transaction_type: String,
      pub amount: i64,
  }
  ```
- **`TryFrom<ResponseRouterData<Execute, SpreedlyRefundResponse, RefundsData, RefundsResponseData>> for RefundsRouterData<ExecuteResponse>` Implementation**:
  ```rust
  impl TryFrom<ResponseRouterData<Execute, SpreedlyRefundResponse, RefundsData, RefundsResponseData>> for RefundsRouterData<ExecuteResponse> {
      type Error = error_stack::Report<errors::ConnectorError>;
      fn try_from(item: ResponseRouterData<Execute, SpreedlyRefundResponse, RefundsData, RefundsResponseData>) -> Result<Self, Self::Error> {
          let refund_status = if item.response.transaction.succeeded {
              common_enums::RefundStatus::Success
          } else {
              common_enums::RefundStatus::Failure
          };
          
          Ok(Self {
              response: ExecuteResponse {
                  amount: item.response.transaction.amount,
                  connector_refund_id: item.response.transaction.token,
                  refund_status,
              },
              ..Default::default()
          })
      }
  }
  ```

### 6.4 Flow: Payment Sync
- **Hyperswitch Trait**: `ConnectorIntegration<PSync, PaymentsSyncData, PaymentsResponseData>`
- **Connector API Endpoint(s)**:
  - Method: GET
  - URL Path: `/v1/transactions/{transaction_token}.json`
- **Amount Handling**: Uses minor units (cents) - no conversion needed

#### 6.4.1 Response Transformation (`transformers.rs`)
- **`SpreedlyPSyncResponse` Struct Definition**:
  ```rust
  #[derive(Debug, Deserialize)]
  pub struct SpreedlyPSyncResponse {
      pub transaction: SpreedlyTransactionResponse,
  }
  ```
- **`TryFrom<ResponseRouterData<PSync, SpreedlyPSyncResponse, PaymentsSyncData, PaymentsResponseData>> for PaymentsSyncRouterData` Implementation**:
  ```rust
  impl TryFrom<ResponseRouterData<PSync, SpreedlyPSyncResponse, PaymentsSyncData, PaymentsResponseData>> for PaymentsSyncRouterData {
      type Error = error_stack::Report<errors::ConnectorError>;
      fn try_from(item: ResponseRouterData<PSync, SpreedlyPSyncResponse, PaymentsSyncData, PaymentsResponseData>) -> Result<Self, Self::Error> {
          let status = if item.response.transaction.succeeded {
              common_enums::AttemptStatus::Charged
          } else {
              common_enums::AttemptStatus::Failure
          };
          
          Ok(Self {
              status,
              response: item.info.clone(),
              amount_received: item.response.transaction.amount,
              router_data: item.data,
              connector_transaction_id: Some(item.response.transaction.token),
              ..Default::default()
          })
      }
  }
  ```

## 7. Connector Specifications (`ConnectorSpecifications` in `spreedly.rs`)
- **`get_connector_about()`**:
  ```rust
  fn get_connector_about(&self) -> types::ConnectorAbout {
      ConnectorInfo {
          name: "Spreedly",
          description: "A global payments orchestration platform that enables businesses to connect with multiple payment gateways and services through a single API.",
      }
  }
  ```
- **`get_supported_payment_methods()`**:
  ```rust
  fn get_supported_payment_methods(&self) -> types::SupportedPaymentMethods {
      types::SupportedPaymentMethods {
          payment_methods: vec![SupportedPaymentMethodsData {
              payment_method: common_enums::PaymentMethod::Card,
              payment_method_types: Some(vec![
                  common_enums::PaymentMethodType::Credit,
                  common_enums::PaymentMethodType::Debit,
              ]),
              card_networks: Some(vec![
                  common_enums::CardNetwork::Visa,
                  common_enums::CardNetwork::Mastercard,
                  common_enums::CardNetwork::AmericanExpress,
                  common_enums::CardNetwork::DinersClub,
                  common_enums::CardNetwork::Discover,
                  common_enums::CardNetwork::JCB,
              ]),
              bank_transfer_types: None,
              ..Default::default()
          }],
      }
  }
  ```
- **`get_supported_webhook_flows()`**: N/A for now

## 8. Webhook Handling (To be implemented in a future phase)

## 9. Configuration Details
### 9.1 Backend (`development.toml`)
```toml
[spreedly]
base_url = "https://core.spreedly.com/v1"
```

### 9.2 Test Authentication (`sample_auth.toml`)
```toml
# Add Spreedly credentials here. KEEP THESE SECRET AND NEVER COMMIT REAL CREDENTIALS.
[spreedly]
environment_key = "ENVIRONMENT_KEY" # Replace with sandbox environment key for testing
access_secret = "ACCESS_SECRET" # Replace with sandbox access secret for testing
```
