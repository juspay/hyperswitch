# Optimized Implementation Plan: {{CONNECTOR_NAME}} Integration

This implementation plan provides a detailed, step-by-step approach for integrating {{CONNECTOR_NAME}} with Hyperswitch. Each step is designed to be independently compilable, self-contained, and complete, allowing for systematic and verifiable progression.

## Phase A: Preparation & Setup

- [ ] **Step A1: Environment & Documentation Preparation**
  - **Task**: Verify development environment and review connector documentation
  - **Files**: N/A (Documentation review)
  - **Step Dependencies**: None
  - **Acceptance Criteria**: Development environment ready and connector API understanding established
  - **User Instructions**:
    - Verify Rust nightly toolchain: `rustup toolchain install nightly && rustup default nightly`
    - Review {{CONNECTOR_NAME}} API documentation at {{CONNECTOR_API_DOCS_URL}}
    - Review technical specification in `tech-spec.md`, focusing on Sections 3 (Authentication), 4 (Error Handling), 5 (API Endpoints)

- [ ] **Step A2: Generate Connector Files**
  - **Task**: Generate and organize boilerplate connector files
  - **Files**:
    - `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs` (created)
    - `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs` (created)
    - `crates/router/tests/connectors/{{connector-name-lowercase}}.rs` (moved)
  - **Step Dependencies**: Step A1
  - **Acceptance Criteria**: All template files are created and in the correct locations
  - **User Instructions**:
    ```bash
    # Generate connector files
    sh scripts/add_connector.sh {{connector-name-lowercase}} {{connector-base-url}}
    
    # Move test file to correct location
    mkdir -p crates/router/tests/connectors/
    mv crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/test.rs crates/router/tests/connectors/{{connector-name-lowercase}}.rs
    ```

## Phase B: Authentication & Error Handling Implementation

- [ ] **Step B1: Implement Authentication & Error Structures**
  - **Task**: Implement authentication and error handling structures
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs`
  - **Step Dependencies**: Step A2
  - **Acceptance Criteria**: Authentication and error structures defined and compiling
  - **Implementation Details**:
    ```rust
    // Auth structure with TryFrom implementation
    #[derive(Debug, Clone, Serialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}AuthType {
        // Authentication fields as specified in tech-spec.md Section 3
        // e.g., api_key, merchant_id, etc.
    }
    
    impl TryFrom<&ConnectorAuthType> for {{CONNECTOR_PASCAL_CASE}}AuthType {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(auth_type: &ConnectorAuthType) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 3
        }
    }
    
    // Error response structure
    #[derive(Debug, Deserialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}ErrorResponse {
        // Error fields as specified in tech-spec.md Section 4
        // e.g., code, message, details, etc.
    }
    ```

## Phase C: Connector Base Implementation

- [ ] **Step C1: Implement Connector Common Functionality**
  - **Task**: Define connector struct and implement ConnectorCommon trait
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs`
  - **Step Dependencies**: Step B1
  - **Acceptance Criteria**: Connector struct and ConnectorCommon trait implemented and compiling
  - **Implementation Details**:
    ```rust
    #[derive(Debug, Clone)]
    pub struct {{CONNECTOR_PASCAL_CASE}};
    
    impl ConnectorCommon for {{CONNECTOR_PASCAL_CASE}} {
        fn id(&self) -> &'static str {
            // Implementation as specified in tech-spec.md Section 5
        }
        
        fn common_get_content_type(&self) -> &'static str {
            // Implementation as specified in tech-spec.md Section 5
        }
        
        fn base_url<'a>(&self, connectors: &'a Settings) -> &'a str {
            // Implementation as specified in tech-spec.md Section 5
        }
        
        fn get_auth_header(&self, auth_type: &ConnectorAuthType) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
            // Implementation as specified in tech-spec.md Section 3
        }
        
        fn build_error_response(&self, res: Response) -> CustomResult<ErrorResponse, errors::ConnectorError> {
            // Implementation as specified in tech-spec.md Section 4
        }
    }
    ```

## Phase D: Core Payment Status Implementation

- [ ] **Step D1: Implement Payment Status Handling**
  - **Task**: Define payment status enum and implement conversion to Hyperswitch status
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs`
  - **Step Dependencies**: Step B1
  - **Acceptance Criteria**: Payment status enum and conversion implemented and compiling
  - **Implementation Details**:
    ```rust
    #[derive(Debug, Deserialize)]
    pub enum {{CONNECTOR_PASCAL_CASE}}PaymentStatus {
        // Status variants as specified in tech-spec.md Section 6.X.2
        // e.g., #[serde(rename = "succeeded")] Success,
    }
    
    impl From<{{CONNECTOR_PASCAL_CASE}}PaymentStatus> for common_enums::AttemptStatus {
        fn from(status: {{CONNECTOR_PASCAL_CASE}}PaymentStatus) -> Self {
            // Implementation as specified in tech-spec.md Section 6.X.2
        }
    }
    ```

## Phase E: Authorize Flow Implementation

- [ ] **Step E1: Implement Authorize Request Structures & Conversion**
  - **Task**: Define authorize request structures and implement conversion from Hyperswitch data
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs`
  - **Step Dependencies**: Steps D1
  - **Acceptance Criteria**: Request structures and conversion implemented and compiling
  - **Implementation Details**:
    ```rust
    // Optional helper structures (if needed)
    #[derive(Debug, Serialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}CardData {
        // Card fields as specified in tech-spec.md Section 6.X.1
    }
    
    #[derive(Debug, Serialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}Address {
        // Address fields as specified in tech-spec.md Section 6.X.1
    }
    
    // Main authorize request structure
    #[derive(Debug, Serialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}AuthorizeRequest {
        // Request fields as specified in tech-spec.md Section 6.X.1
    }
    
    impl TryFrom<&{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsAuthorizeData>> for {{CONNECTOR_PASCAL_CASE}}AuthorizeRequest {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: &{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsAuthorizeData>) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 6.X.1
        }
    }
    ```

- [ ] **Step E2: Implement Authorize Response Structures & Conversion**
  - **Task**: Define authorize response structures and implement conversion to Hyperswitch data
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs`
  - **Step Dependencies**: Step E1
  - **Acceptance Criteria**: Response structures and conversion implemented and compiling
  - **Implementation Details**:
    ```rust
    #[derive(Debug, Deserialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}AuthorizeResponse {
        // Response fields as specified in tech-spec.md Section 6.X.2
    }
    
    impl TryFrom<ResponseRouterData<Authorize, {{CONNECTOR_PASCAL_CASE}}AuthorizeResponse, PaymentsAuthorizeData, PaymentsResponseData>> for PaymentsAuthorizeRouterData {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: ResponseRouterData<Authorize, {{CONNECTOR_PASCAL_CASE}}AuthorizeResponse, PaymentsAuthorizeData, PaymentsResponseData>) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 6.X.2
        }
    }
    ```

- [ ] **Step E3: Implement Authorize ConnectorIntegration**
  - **Task**: Implement ConnectorIntegration trait for Authorize flow
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs`
  - **Step Dependencies**: Steps C1, E2
  - **Acceptance Criteria**: ConnectorIntegration for Authorize implemented and compiling
  - **Implementation Details**:
    ```rust
    impl ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData> for {{CONNECTOR_PASCAL_CASE}} {
        // Implementation methods as specified in tech-spec.md Section 6.X.3
        fn get_headers(&self, req: &PaymentsAuthorizeRouterData, connectors: &Settings) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
            // Implementation
        }
        
        fn get_content_type(&self) -> &'static str {
            // Implementation
        }
        
        fn get_url(&self, req: &PaymentsAuthorizeRouterData, connectors: &Settings) -> CustomResult<String, errors::ConnectorError> {
            // Implementation
        }
        
        fn get_request_body(&self, req: &PaymentsAuthorizeRouterData, connectors: &Settings) -> CustomResult<RequestContent, errors::ConnectorError> {
            // Implementation
        }
        
        fn handle_response(
            &self,
            data: &PaymentsAuthorizeRouterData,
            res: Response,
        ) -> CustomResult<PaymentsAuthorizeRouterData, errors::ConnectorError> {
            // Implementation
        }
        
        fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse, errors::ConnectorError> {
            // Implementation
        }
    }
    ```

## Phase F: Registration & Configuration

- [ ] **Step F1: Register Connector in Core Enums**
  - **Task**: Add connector to core enums and implement necessary traits
  - **Files**: `crates/common_enums/src/connector_enums.rs`
  - **Step Dependencies**: Step C1
  - **Acceptance Criteria**: Connector added to core enums and compiling
  - **Implementation Details**:
    ```rust
    // Add to Connector enum
    pub enum Connector {
        // ... existing variants
        {{CONNECTOR_PASCAL_CASE}},
    }
    
    // Add to RoutableConnectors enum if applicable
    pub enum RoutableConnectors {
        // ... existing variants
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
    
    // Update other necessary trait implementations
    ```

- [ ] **Step F2: Configure Connector Settings**
  - **Task**: Add connector configuration to development and test files
  - **Files**:
    - `crates/connector_configs/toml/development.toml`
    - `crates/router/tests/connectors/sample_auth.toml`
  - **Step Dependencies**: Steps B1, C1
  - **Acceptance Criteria**: Configuration added and correct
  - **Implementation Details**:
    ```toml
    # In development.toml
    [connectors.{{connector-name-lowercase}}]
    base_url = "{{connector-base-url}}"
    # Additional configuration as specified in tech-spec.md Section 9.1
    
    # In sample_auth.toml
    [{{connector-name-lowercase}}]
    # Authentication details as specified in tech-spec.md Section 9.2
    # e.g., api_key = "test_api_key"
    ```

## Phase G: Testing Authorize Flow

- [ ] **Step G1: Implement Authorize Test**
  - **Task**: Implement test for authorize payment flow
  - **Files**: `crates/router/tests/connectors/{{connector-name-lowercase}}.rs`
  - **Step Dependencies**: Steps E3, F2
  - **Acceptance Criteria**: Test implementation compiles and tests the authorize flow
  - **Implementation Details**:
    ```rust
    // Test data setup
    fn get_data() -> Data {
        // Implementation as specified in tech-spec.md Section 10
    }
    
    fn get_auth_token() -> ConnectorAuthType {
        // Implementation as specified in tech-spec.md Section 10
    }
    
    // Optional: Additional setup functions
    fn get_default_payment_info() -> PaymentInfo {
        // Implementation
    }
    
    // Test case
    #[test]
    fn test_{{connector-name-lowercase}}_authorize_success() {
        // Implementation as specified in tech-spec.md Section 10
    }
    ```

- [ ] **Step G2: Verify Authorize Flow**
  - **Task**: Run and validate the authorize test
  - **Files**: N/A (Test execution)
  - **Step Dependencies**: Step G1
  - **Acceptance Criteria**: Test passes successfully
  - **User Instructions**:
    ```bash
    export CONNECTOR_AUTH_FILE_PATH="crates/router/tests/connectors/sample_auth.toml"
    cargo test --package router --test connectors -- {{connector-name-lowercase}}::test_{{connector-name-lowercase}}_authorize_success --test-threads=1
    ```

## Phase H: Capture Flow Implementation

- [ ] **Step H1: Implement Capture Data Structures**
  - **Task**: Implement request/response structures and conversions for capture flow
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs`
  - **Step Dependencies**: Step E2
  - **Acceptance Criteria**: Capture structures and conversions implemented and compiling
  - **Implementation Details**:
    ```rust
    // Capture request structure
    #[derive(Debug, Serialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}CaptureRequest {
        // Request fields as specified in tech-spec.md Section 6.Y.1
    }
    
    impl TryFrom<&{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsCaptureData>> for {{CONNECTOR_PASCAL_CASE}}CaptureRequest {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: &{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsCaptureData>) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 6.Y.1
        }
    }
    
    // Capture response structure
    #[derive(Debug, Deserialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}CaptureResponse {
        // Response fields as specified in tech-spec.md Section 6.Y.2
    }
    
    impl TryFrom<ResponseRouterData<Capture, {{CONNECTOR_PASCAL_CASE}}CaptureResponse, PaymentsCaptureData, PaymentsCaptureResponseData>> for PaymentsCaptureRouterData {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: ResponseRouterData<Capture, {{CONNECTOR_PASCAL_CASE}}CaptureResponse, PaymentsCaptureData, PaymentsCaptureResponseData>) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 6.Y.2
        }
    }
    ```

- [ ] **Step H2: Implement Capture ConnectorIntegration**
  - **Task**: Implement ConnectorIntegration trait for Capture flow
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs`
  - **Step Dependencies**: Step H1
  - **Acceptance Criteria**: ConnectorIntegration for Capture implemented and compiling
  - **Implementation Details**:
    ```rust
    impl ConnectorIntegration<Capture, PaymentsCaptureData, PaymentsCaptureResponseData> for {{CONNECTOR_PASCAL_CASE}} {
        // Implementation methods as specified in tech-spec.md Section 6.Y.3
        fn get_headers(&self, req: &PaymentsCaptureRouterData, connectors: &Settings) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
            // Implementation
        }
        
        fn get_content_type(&self) -> &'static str {
            // Implementation
        }
        
        fn get_url(&self, req: &PaymentsCaptureRouterData, connectors: &Settings) -> CustomResult<String, errors::ConnectorError> {
            // Implementation
        }
        
        fn get_request_body(&self, req: &PaymentsCaptureRouterData, connectors: &Settings) -> CustomResult<RequestContent, errors::ConnectorError> {
            // Implementation
        }
        
        fn handle_response(
            &self,
            data: &PaymentsCaptureRouterData,
            res: Response,
        ) -> CustomResult<PaymentsCaptureRouterData, errors::ConnectorError> {
            // Implementation
        }
        
        fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse, errors::ConnectorError> {
            // Implementation
        }
    }
    ```

- [ ] **Step H3: Implement Capture Test**
  - **Task**: Implement test for capture payment flow
  - **Files**: `crates/router/tests/connectors/{{connector-name-lowercase}}.rs`
  - **Step Dependencies**: Step H2
  - **Acceptance Criteria**: Test implementation compiles and tests the capture flow
  - **Implementation Details**:
    ```rust
    #[test]
    fn test_{{connector-name-lowercase}}_capture_success() {
        // Implementation as specified in tech-spec.md Section 10
    }
    ```

- [ ] **Step H4: Verify Capture Flow**
  - **Task**: Run and validate the capture test
  - **Files**: N/A (Test execution)
  - **Step Dependencies**: Step H3
  - **Acceptance Criteria**: Test passes successfully
  - **User Instructions**:
    ```bash
    export CONNECTOR_AUTH_FILE_PATH="crates/router/tests/connectors/sample_auth.toml"
    cargo test --package router --test connectors -- {{connector-name-lowercase}}::test_{{connector-name-lowercase}}_capture_success --test-threads=1
    ```

## Phase I: Payment Sync Flow Implementation

- [ ] **Step I1: Implement Sync Data Structures**
  - **Task**: Implement request/response structures and conversions for payment sync flow
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs`
  - **Step Dependencies**: Step E2
  - **Acceptance Criteria**: Sync structures and conversions implemented and compiling
  - **Implementation Details**:
    ```rust
    // Sync request structure (if needed)
    #[derive(Debug, Serialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}PsyncRequest {
        // Request fields as specified in tech-spec.md Section 6.Z.1
    }
    
    impl TryFrom<&{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsSyncData>> for {{CONNECTOR_PASCAL_CASE}}PsyncRequest {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: &{{CONNECTOR_PASCAL_CASE}}RouterData<&PaymentsSyncData>) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 6.Z.1
        }
    }
    
    // Sync response structure
    #[derive(Debug, Deserialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}PsyncResponse {
        // Response fields as specified in tech-spec.md Section 6.Z.2
    }
    
    impl TryFrom<ResponseRouterData<PSync, {{CONNECTOR_PASCAL_CASE}}PsyncResponse, PaymentsSyncData, PaymentsSyncResponseData>> for PaymentsSyncRouterData {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: ResponseRouterData<PSync, {{CONNECTOR_PASCAL_CASE}}PsyncResponse, PaymentsSyncData, PaymentsSyncResponseData>) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 6.Z.2
        }
    }
    ```

- [ ] **Step I2: Implement Sync ConnectorIntegration**
  - **Task**: Implement ConnectorIntegration trait for Payment Sync flow
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs`
  - **Step Dependencies**: Step I1
  - **Acceptance Criteria**: ConnectorIntegration for PSync implemented and compiling
  - **Implementation Details**:
    ```rust
    impl ConnectorIntegration<PSync, PaymentsSyncData, PaymentsSyncResponseData> for {{CONNECTOR_PASCAL_CASE}} {
        // Implementation methods as specified in tech-spec.md Section 6.Z.3
        fn get_headers(&self, req: &PaymentsSyncRouterData, connectors: &Settings) -> CustomResult<Vec<(String, request::Maskable<String>)>, errors::ConnectorError> {
            // Implementation
        }
        
        fn get_content_type(&self) -> &'static str {
            // Implementation
        }
        
        fn get_url(&self, req: &PaymentsSyncRouterData, connectors: &Settings) -> CustomResult<String, errors::ConnectorError> {
            // Implementation
        }
        
        fn get_request_body(&self, req: &PaymentsSyncRouterData, connectors: &Settings) -> CustomResult<RequestContent, errors::ConnectorError> {
            // Implementation
        }
        
        fn handle_response(
            &self,
            data: &PaymentsSyncRouterData,
            res: Response,
        ) -> CustomResult<PaymentsSyncRouterData, errors::ConnectorError> {
            // Implementation
        }
        
        fn get_error_response(&self, res: Response) -> CustomResult<ErrorResponse, errors::ConnectorError> {
            // Implementation
        }
    }
    ```

- [ ] **Step I3: Implement Sync Test**
  - **Task**: Implement test for payment sync flow
  - **Files**: `crates/router/tests/connectors/{{connector-name-lowercase}}.rs`
  - **Step Dependencies**: Step I2
  - **Acceptance Criteria**: Test implementation compiles and tests the sync flow
  - **Implementation Details**:
    ```rust
    #[test]
    fn test_{{connector-name-lowercase}}_psync_success() {
        // Implementation as specified in tech-spec.md Section 10
    }
    ```

- [ ] **Step I4: Verify Sync Flow**
  - **Task**: Run and validate the sync test
  - **Files**: N/A (Test execution)
  - **Step Dependencies**: Step I3
  - **Acceptance Criteria**: Test passes successfully
  - **User Instructions**:
    ```bash
    export CONNECTOR_AUTH_FILE_PATH="crates/router/tests/connectors/sample_auth.toml"
    cargo test --package router --test connectors -- {{connector-name-lowercase}}::test_{{connector-name-lowercase}}_psync_success --test-threads=1
    ```

## Phase J: Refund Flow Implementation

- [ ] **Step J1: Implement Refund Data Structures**
  - **Task**: Implement request/response structures and conversions for refund flow
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs`
  - **Step Dependencies**: Step E2
  - **Acceptance Criteria**: Refund structures and conversions implemented and compiling
  - **Implementation Details**:
    ```rust
    // Refund request structure
    #[derive(Debug, Serialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}RefundRequest {
        // Request fields as specified in tech-spec.md Section 6.W.1
    }
    
    impl TryFrom<&{{CONNECTOR_PASCAL_CASE}}RouterData<&RefundsData>> for {{CONNECTOR_PASCAL_CASE}}RefundRequest {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: &{{CONNECTOR_PASCAL_CASE}}RouterData<&RefundsData>) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 6.W.1
        }
    }
    
    // Refund response structure
    #[derive(Debug, Deserialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}RefundResponse {
        // Response fields as specified in tech-spec.md Section 6.W.2
    }
    
    impl TryFrom<ResponseRouterData<Execute, {{CONNECTOR_PASCAL_CASE}}RefundResponse, RefundsData, RefundsResponseData>> for RefundsRouterData<RefundsResponseData> {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: ResponseRouterData<Execute, {{CONNECTOR_PASCAL_CASE}}RefundResponse, RefundsData, RefundsResponseData>) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 6.W.2
        }
    }
    ```

- [ ] **Step J2: Implement Refund ConnectorIntegration**
  - **Task**: Implement ConnectorIntegration trait for Refund flow
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs`
  - **Step Dependencies**: Step J1
  - **Acceptance Criteria**: ConnectorIntegration for Refund implemented and compiling
  - **Implementation Details**:
    ```rust
    impl ConnectorIntegration<Execute, RefundsData, RefundsResponseData> for {{CONNECTOR_PASCAL_CASE}} {
        // Implementation methods as specified in tech-spec.md Section 6.W.3
    }
    ```

- [ ] **Step J3: Implement Refund Sync Data Structures**
  - **Task**: Implement request/response structures and conversions for refund sync flow
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}/transformers.rs`
  - **Step Dependencies**: Step J1
  - **Acceptance Criteria**: Refund sync structures and conversions implemented and compiling
  - **Implementation Details**:
    ```rust
    // Refund sync request structure (if needed)
    #[derive(Debug, Serialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}RefundSyncRequest {
        // Request fields as specified in tech-spec.md Section 6.V.1
    }
    
    impl TryFrom<&{{CONNECTOR_PASCAL_CASE}}RouterData<&RefundSyncData>> for {{CONNECTOR_PASCAL_CASE}}RefundSyncRequest {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: &{{CONNECTOR_PASCAL_CASE}}RouterData<&RefundSyncData>) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 6.V.1
        }
    }
    
    // Refund sync response structure
    #[derive(Debug, Deserialize)]
    pub struct {{CONNECTOR_PASCAL_CASE}}RefundSyncResponse {
        // Response fields as specified in tech-spec.md Section 6.V.2
    }
    
    impl TryFrom<ResponseRouterData<RSync, {{CONNECTOR_PASCAL_CASE}}RefundSyncResponse, RefundSyncData, RefundsResponseData>> for RefundsRouterData<RefundsResponseData> {
        type Error = error_stack::Report<errors::ConnectorError>;
        fn try_from(item: ResponseRouterData<RSync, {{CONNECTOR_PASCAL_CASE}}RefundSyncResponse, RefundSyncData, RefundsResponseData>) -> Result<Self, Self::Error> {
            // Implementation as specified in tech-spec.md Section 6.V.2
        }
    }
    ```

- [ ] **Step J4: Implement Refund Sync ConnectorIntegration**
  - **Task**: Implement ConnectorIntegration trait for Refund Sync flow
  - **Files**: `crates/hyperswitch_connectors/src/connectors/{{connector-name-lowercase}}.rs`
  - **Step Dependencies**: Step J3
  - **Acceptance Criteria**: ConnectorIntegration for RSync implemented and compiling
  - **Implementation Details**:
    ```rust
    impl ConnectorIntegration<RSync, RefundSyncData, RefundsResponseData> for {{CONNECTOR_PASCAL_CASE}} {
        // Implementation methods as specified in tech-spec.md Section 6.V.3
    }
    ```

- [ ] **Step J5: Implement Refund Tests**
  - **Task**: Implement tests for refund and refund sync flows
  - **Files**: `crates/router/tests/connectors/{{connector-name-lowercase}}.rs`
  - **Step Dependencies**: Steps J2, J4
  - **Acceptance Criteria**: Test implementations compile and test the refund flows
  - **Implementation Details**:
    ```rust
    #[test]
    fn test_{{connector-name-lowercase}}_refund_success() {
        // Implementation as specified in tech-spec.md Section 10
    }
    
    #[test]
    fn test_{{connector-name-lowercase}}_refund_sync_success() {
        // Implementation as specified in tech-spec.md Section 10
    }
    ```

- [ ] **Step J6: Verify Refund Flows**
  - **Task**: Run and validate the refund tests
  - **Files**: N/A (Test execution)
  - **Step Dependencies**: Step J
