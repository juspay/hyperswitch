# Guide to Integrating a Connector

## Table of Contents

1. [Introduction](#introduction)  
2. [Prerequisites](#prerequisites)  
3. [Development Environment Setup & Configuration](#development-environment-setup--configuration)  
4. [Create a Connector](#create-a-connector)  
5. [Test the Connection](#test-the-connection)  
6. [Folder Structure After Running the Script](#folder-structure-after-running-the-script)  
7. [Common Payment Flow Types](#common-payment-flow-types)  
8. [Integrate a New Connector](#integrate-a-new-connector)  
9. [Code Walkthrough](#code-walkthrough)  
10. [Error Handling in Hyperswitch Connectors](#error-handling-in-hyperswitch-connectors)  
11. [Implementing the Connector Interface](#implementing-the-connector-interface)  
12. [ConnectorCommon: The Foundation Trait](#connectorcommon-the-foundation-trait)  
13. [ConnectorIntegration – The Payment Flow Orchestrator](#connectorintegration--the-payment-flow-orchestrator)  
14. [Method-by-Method Breakdown](#method-by-method-breakdown)  
15. [Connector Traits Overview](#connector-traits-overview)  
16. [Derive Traits](#derive-traits)  
17. [Connector Utility Functions](#connector-utility-functions)  
18. [Connector Configuration for Control Center Integration](#connector-configuration-for-control-center-integration)  
19. [Control Center Frontend Integration](#control-center-frontend-integration)  
20. [Test the Connector Integration](#test-the-connector-integration)  


## Introduction

This guide provides instructions on integrating a new connector with Router, from setting up the environment to implementing API interactions. In this document you’ll learn how to:

* Scaffold a new connector template
* Define Rust request/response types directly from your PSP’s JSON schema
* Implement transformers and the `ConnectorIntegration` trait for both standard auth and tokenization-first flows
* Enforce PII best practices (Secret wrappers, common\_utils::pii types) and robust error-handling
* Update the Control-Center (ConnectorTypes.res, ConnectorUtils.res, icons)
* Validate your connector with end-to-end tests

By the end, you’ll learn how to create a fully functional, production-ready connector—from blank slate to live in the Control-Center.

## Prerequisites

* Before you begin, ensure you’ve completed the initial setup in our [Hyperswitch Contributor Guide](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/docs/CONTRIBUTING.md?plain=1#L1), which covers cloning, tool installation, and access.
* You should also understanding [connectors and payment methods](https://hyperswitch.io/pm-list).
* Familiarity with the Connector API you’re integrating
* A locally set up and running Router repository
* API credentials for testing (sign up for sandbox/UAT credentials on the connector’s website).
* Need help? Join the [Hyperswitch Slack Channel](https://inviter.co/hyperswitch-slack). We also have weekly office hours every Thursday at 8:00 AM PT (11:00 AM ET, 4:00 PM BST, 5:00 PM CEST, and 8:30 PM IST). Link to office hours are shared in the **#general channel**.

## Development Environment Setup & Configuration

This guide will walk you through your environment setup and configuration.

### Clone the Hyperswitch monorepo

```bash
git clone git@github.com:juspay/hyperswitch.git
cd hyperswitch
```

### Rust Environment & Dependencies Setup

Before running Hyperswitch locally, make sure your Rust environment and system dependencies are properly configured.

**Follow the guide**:

[Configure Rust and install required dependencies based on your OS](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-a-rust-environment-and-other-dependencies)

**Quick links by OS**:
* [Ubuntu-based systems](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-dependencies-on-ubuntu-based-systems)
* [Windows (WSL2)](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-dependencies-on-windows-ubuntu-on-wsl2)
* [Windows (native)](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-dependencies-on-windows)
* [macOS](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-dependencies-on-macos)

**All OS Systems**:
* [Set up the database](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-the-database)

* Set up the Rust nightly toolchain installed for code formatting:

```bash
rustup toolchain install nightly
```

* Install [Protobuf](https://protobuf.dev/installation/)

Install cargo-generate for creating project templates:

```bash
cargo install cargo-generate
```

If you've completed the setup, you should now have:

* ✅ Rust & Cargo
* ✅ `cargo-generate`
* ✅ PostgreSQL (with a user and database created)
* ✅ Redis
* ✅ `diesel_cli`
* ✅ The `just` command runner
* ✅ Database migrations applied
* ✅ Set up the Rust nightly toolchain
* ✅ Installed Protobuf

Compile and run the application using cargo:

```bash
cargo run
```

## Create a Connector
From the root of the project, generate a new connector by running the following command. Use a single-word name for your `ConnectorName`:

```bash
sh scripts/add_connector.sh <ConnectorName> <ConnectorBaseUrl>
```

When you run the script, you should see that some files were created

```bash
# Done! New project created /absolute/path/hyperswitch/crates/hyperswitch_connectors/src/connectors/connectorname
```

> ⚠️ **Warning**  
> Don’t be alarmed if you see test failures at this stage.  
> Tests haven’t been implemented for your new connector yet, so failures are expected.  
> You can safely ignore output like this:
>
> ```bash
> test result: FAILED. 0 passed; 20 failed; 0 ignored; 0 measured; 1759 filtered out; finished in 0.10s
> ```
> You can also ignore GRPC errors too.

## Test the connection 
Once you've successfully created your connector using the `add_connector.sh` script, you can verify the integration by starting the Hyperswitch Router Service:

```bash
cargo r
```

This launches the router application locally on `port 8080`, providing access to the complete Hyperswitch API. You can now test your connector implementation by making HTTP requests to the payment endpoints for operations like:

- Payment authorization and capture
- Payment synchronization
- Refund processing
- Webhook handling

Once your connector logic is implemented, this environment lets you ensure it behaves correctly within the Hyperswitch orchestration flow—before moving to staging or production.

### Verify Server Health

Once the Hyperswitch Router Service is running, you can verify it's operational by checking the health endpoint in a separate terminal window:

```bash
curl --head --request GET 'http://localhost:8080/health'
```
> **Action Item**  
> After creating the connector, run a health check to ensure everything is working smoothly.

### Folder Structure After Running the Script
When you run the script, it creates a specific folder structure for your new connector. Here's what gets generated:

**Main Connector Files**

The script creates the primary connector structure in the hyperswitch_connectors crate:

crates/hyperswitch_connectors/src/connectors/  
├── <connector_name>/  
│   └── transformers.rs  
└── <connector_name>.rs

#### Test Files

The script also generates test files in the router crate:

crates/router/tests/connectors/  
└── <connector_name>.rs 

**What Each File Contains**

- `<connector_name>.rs`: The main connector implementation file where you implement the connector traits
- `transformers.rs`: Contains data structures and conversion logic between Hyperswitch's internal format and your payment processor's API format
- **Test file**: [Contains boilerplate test cases for your connector](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/connector-template/test.rs#L1-L36).

## Common Payment Flow Types

As you build your connector, you’ll encounter different payment flow patterns.  
This section gives you:

- A quick reference table for all flows  
- Examples of the two most common patterns: **Tokenization‑first** and **Direct Authorization**

> For full details, see [Connector Payment Flow documentation](https://docs.hyperswitch.io/learn-more/hyperswitch-architecture/connector-payment-flows) or ask us in Slack.

---

### 1. Flow Summary Table

| Flow Name           | Description                                      | Implementation in Hyperswitch |
|---------------------|--------------------------------------------------|--------------------------------|
| **Access Token**      | Obtain OAuth access token                        | [crates/hyperswitch_interfaces/src/types.rs#L7](https://github.com/juspay/hyperswitch/blob/06dc66c62e33c1c56c42aab18a7959e1648d6fae/crates/hyperswitch_interfaces/src/types.rs#L7) |
| **Tokenization**      | Exchange credentials for a payment token         | [crates/hyperswitch_interfaces/src/types.rs#L148](https://github.com/juspay/hyperswitch/blob/06dc66c62e33c1c56c42aab18a7959e1648d6fae/crates/hyperswitch_interfaces/src/types.rs#L148) |
| **Customer Creation** | Create or update customer records                | [crates/router/src/types.rs#L40](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L40) |
| **Pre‑Processing**    | Validation or enrichment before auth             | [crates/router/src/types.rs#L41](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L41) |
| **Authorization**     | Authorize and immediately capture payment        | [crates/hyperswitch_interfaces/src/types.rs#L12](https://github.com/juspay/hyperswitch/blob/06dc66c62e33c1c56c42aab18a7959e1648d6fae/crates/hyperswitch_interfaces/src/types.rs#L12) |
| **Authorization‑Only**| Authorize payment for later capture              | [crates/router/src/types.rs#L39](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L39) |
| **Capture**           | Capture a previously authorized payment          | [crates/router/src/types.rs#L39](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L39) |
| **Refund**            | Issue a refund                                   | [crates/router/src/types.rs#L44](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L44) |
| **Webhook Handling**  | Process asynchronous events from PSP             | [crates/router/src/types.rs#L45](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L45) |

---
### Flow Type Definitions

Each flow type corresponds to specific request/response data structures and connector integration patterns. All flows follow a standardized pattern with associated:

- **Request data types** (e.g., `PaymentsAuthorizeData`)
- **Response data types** (e.g., `PaymentsResponseData`)
- **Router data wrappers** for connector communication

### 2. Pattern: Tokenization‑First

Some PSPs require payment data to be tokenized before it can be authorized.  
This is a **two‑step process**:  

1. **Tokenization** – e.g., Billwerk’s implementation:  
   - [Tokenization](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L178-L271)  
   - [Authorization](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L273-L366)  

2. **Authorization** – Uses the returned token rather than raw payment details.  

> Most PSPs don’t require this; see the next section for direct authorization.

---

### 3. Pattern: Direct Authorization

Many connectors skip tokenization and send payment data directly in the authorization request.  

- **Authorize.net** – [code](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/authorizedotnet.rs#L401-L497)  
  Builds `CreateTransactionRequest` directly from payment data in `get_request_body()`.  

- **Helcim** – [code](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/helcim.rs#L295-L385)  
  Chooses purchase (auto‑capture) or preauth endpoint in `get_url()` and processes payment data directly.  

- **Deutsche Bank** – [code](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/deutschebank.rs#L330-L461)  
  Selects flow based on 3DS and payment type (card or direct debit).  

**Key differences from tokenization‑first:**
- Single API call – No separate token step  
- No token storage – No token management required  
- Immediate processing – `get_request_body()` handles payment data directly  

All implement the same `ConnectorIntegration<Authorize, PaymentsAuthorizeData, PaymentsResponseData>` pattern.

## Integrate a New Connector

Integrating a connector is mainly an API integration task. You'll define request and response types and implement required traits.

This section covers card payments via Billwerk. Review the API reference and test APIs before starting. You can leverage these examples for your connector of choice. 

### 1. Build Payment Request and Response from JSON Schema

To generate Rust types from your connector’s OpenAPI or JSON schema, you’ll need to install the [OpenAPI Generator](https://openapi-generator.tech/).

### Example (macOS using Homebrew):

```bash
brew install openapi-generator
```
> 💡 **Note:**  
> On **Linux**, you can install OpenAPI Generator using `apt`, `snap`, or by downloading the JAR from the [official site](https://openapi-generator.tech/docs/installation).  
> On **Windows**, use [Scoop](https://scoop.sh/) or manually download the JAR file.

### 2. **Download the OpenAPI Specification from your connector**

First, obtain the OpenAPI specification from your payment processor's developer documentation. Most processors provide these specifications at standardized endpoints.

```bash
curl -o <ConnectorName>-openapi.json <schema-url>
```
**Specific Example**:

For Billwerk (using their sandbox environment):

```bash
curl -o billwerk-openapi.json https://sandbox.billwerk.com/swagger/v1/swagger.json
```
For other connectors, check their developer documentation for similar endpoints like:

- `/swagger/v1/swagger.json`
- `/openapi.json`
- `/api-docs`

After running the complete command, you'll have:

`crates/hyperswitch_connectors/src/connectors/{CONNECTORNAME}/temp.rs `

This single file contains all the Rust structs and types generated from your payment processor's OpenAPI specification.

The generated `temp.rs` file typically contains:

- **Request structs**: Data structures for API requests
- **Response structs**: Data structures for API responses
- **Enum types**: Status codes, payment methods, error types
- **Nested objects**: Complex data structures used within requests/responses
- **Serde annotations**: Serialization/deserialization attributes.

Otherwise, you can manually define it and create the `crates/hyperswitch_connectors/src/connectors/{CONNECTOR_NAME}/temp.rs ` file. We highly recommend using the `openapi-generator` for ease. 

#### Usage in Connector Development

You can then copy and adapt these generated structs into your connector's `transformers.rs` file, following the pattern shown in the connector integration documentation. The generated code serves as a starting point that you customize for your specific connector implementation needs.

### 3. **Configure Required Environment Variables**

Set up the necessary environment variables for the OpenAPI generation process:

#### Connector name (must match the name used in add_connector.sh script) 

```bash
export CONNECTOR_NAME="ConnectorName"  
``` 

#### Path to the downloaded OpenAPI specification  
```bash
export SCHEMA_PATH="/absolute/path/to/your/connector-openapi.json"
```

## Code Walkthrough

We'll walk through the `transformer.rs` file, and what needs to be implemented.

### 1. **Converts Hyperswitch's internal payment data into your connector's API request format**
 This part of the code takes your internal representation of a payment request, pulls out the token, gathers all the customer and payment fields, and packages them into a clean, JSON-serializable struct ready to send to your connector of choice (in this case Billwerk). You'll have to implement the customer and payment fields, as necessary. 

 The code below extracts customer data and constructs a payment request:

```rust
//TODO: Fill the struct with respective fields
// Auth Struct

impl TryFrom<&ConnectorAuthType> for NadinebillwerkAuthType {
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
Here's an implementation example with the Billwerk connector:

```rust
#[derive(Debug, Serialize)]
pub struct NadinebillwerkCustomerObject {
    handle: Option<id_type::CustomerId>,
    email: Option<Email>,
    address: Option<Secret<String>>,
    address2: Option<Secret<String>>,
    city: Option<String>,
    country: Option<common_enums::CountryAlpha2>,
    first_name: Option<Secret<String>>,
    last_name: Option<Secret<String>>,
}

impl TryFrom<&NadinebillwerkRouterData<&PaymentsAuthorizeRouterData>>
    for NadinebillwerkPaymentsRequest
{
    type Error = error_stack::Report<errors::ConnectorError>;
    fn try_from(
        item: &NadinebillwerkRouterData<&PaymentsAuthorizeRouterData>,
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
            customer: NadinebillwerkCustomerObject {
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

2. **Handle Response Mapping**

Response mapping is a critical component of connector implementation that translates payment processor–specific statuses into Hyperswitch’s standardized internal representation. This ensures consistent payment state management across all integrated payment processors.

**Define Payment Status Enum**

Create an enum that represents all possible payment statuses returned by your payment processor’s API. This enum should match the exact status values specified in your connector’s API documentation.

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
```
The enum uses `#[serde(rename_all = "lowercase")]` to automatically handle JSON serialization/deserialization in the connector’s expected format.

**Implement Status Conversion**

Implement From <ConnectorStatus> for Hyperswitch’s `AttemptStatus` enum. Below is an example implementation:

```rs
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

| Connector Status       | Hyperswitch Status            | Description                          |
|------------------------|-------------------------------|--------------------------------------|
| `Created`, `Pending`   | `AttemptStatus::Pending`      | Payment is being processed           |
| `Authorized`           | `AttemptStatus::Authorized`   | Payment authorized, awaiting capture |
| `Settled`              | `AttemptStatus::Charged`      | Payment successfully completed       |
| `Failed`               | `AttemptStatus::Failure`      | Payment failed or was declined       |
| `Cancelled`            | `AttemptStatus::Voided`       | Payment was cancelled/voided         |

> **Note:** Default status should be `Pending`. Only explicit success or failure from the connector should mark the payment as `Charged` or `Failure`.

3. **Mapping Billwerk API Responses (or any PSPs) to Hyperswitch Internal Specification**

Billwerk, like most payment service providers (PSPs), has its own proprietary API response format with custom fields, naming conventions, and nested structures. However, Hyperswitch is designed to be connector-agnostic: it expects all connectors to normalize external data into a consistent internal format, so it can process payments uniformly across all supported PSPs.

The response struct acts as the translator between these two systems. This process ensures that regardless of which connector you're using, Hyperswitch can process payment responses consistently.

```rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BillwerkPaymentsResponse {
    state: BillwerkPaymentState,
    handle: String,
    error: Option<String>,
    error_state: Option<String>,
}
```
**Key Fields Explained**:

- **state**: Payment status using the enum we defined earlier
- **handle**: Billwerk's unique transaction identifier
- **error & error_state**: Optional error information for failure scenarios


The `try_from` function converts connector-specific, like Billwerk, response data into Hyperswitch's standardized format: 

```rs
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
### Transformation Logic:

- **Error Handling**: Checks for error conditions first and creates appropriate error responses
- **Status Mapping**: Converts BillwerkPaymentState to standardized AttemptStatus using our enum mapping
- **Data Extraction**: Maps PSP-specific fields to Hyperswitch's PaymentsResponseData structure
- **Metadata Preservation**: Ensures important transaction details are retained

#### Critical Response Fields

The transformation populates these essential Hyperswitch fields:

- **resource_id**: Maps to connector transaction ID for future operations
- **connector_response_reference_id**: Preserves PSP's reference for dashboard linking
- **status**: Standardized payment status for consistent processing
- **redirection_data**: Handles 3DS or other redirect flows
- **network_txn_id**: Captures network-level transaction identifiers


### Field Mapping Patterns:

Each critical response field requires specific implementation patterns to ensure consistent behavior across all Hyperswitch connectors. 

- **connector_request_reference_id**: This field carries the merchant’s reference ID and is populated during request construction. It is sent to the PSP to support end-to-end transaction traceability.

```rs
reference: item.router_data.connector_request_reference_id.clone(),
```

- **connector_response_reference_id**: Stores the payment processor’s transaction reference and is used for downstream reconciliation and dashboard visibility. Prefer the PSP's designated reference field if available; otherwise, fall back to the transaction ID. This ensures accurate linkage across merchant dashboards, support tools, and internal systems.


```rs
connector_response_reference_id: item.response.reference.or(Some(item.response.id)),
```

- **resource_id**: Defines the primary resource identifier used for subsequent operations such as captures, refunds, and syncs. Typically sourced from the connector’s transaction ID. If the transaction ID is unavailable, use ResponseId::NoResponseId as a fallback to preserve type safety.

```rs
`resource_id: types::ResponseId::ConnectorTransactionId(item.response.id.clone()),
```

- **redirection_data**: Captures redirection details required for authentication flows such as 3DS. If the connector provides a redirect URL, populate this field accordingly. For advanced flows involving form submissions, construct a `RedirectForm::Form` using the target endpoint, HTTP method, and form fields.

```rs
let redirection_data = item.response.links.redirect.map(|href| {  
    services::RedirectForm::from((href.redirection_url, services::Method::Get))  
});
```

- **network_txn_id**: Stores the transaction identifier issued by the underlying payment network (e.g., Visa, Mastercard). This field is optional but highly useful for advanced reconciliation, chargeback handling, and network-level dispute resolution—especially when the network ID differs from the PSP’s transaction ID.

```rs
network_txn_id: item.response.network_transaction_id.clone(),
```

4. **Error Handling in Hyperswitch Connectors**

Hyperswitch connectors implement a structured error-handling mechanism that categorizes HTTP error responses by type. By distinguishing between client-side errors (4xx) and server-side errors (5xx), the system enables more precise handling strategies tailored to the source of the failure.

**Error Response Structure**

Billwerk defines its error response format to capture failure information from API calls. You can find this in the `transformer.rs file`:

```rs
#[derive(Debug, Serialize, Deserialize)]  
pub struct BillwerkErrorResponse {  
    pub code: Option<i32>,  
    pub error: String,  
    pub message: Option<String>,  
}
```

- **code**: Optional integer error code from Billwerk
- **error**: Required string describing the error
- **message**: Optional additional error messagecode: Optional integer error code from Billwerk
- **error**: Required string describing the error
message: Optional additional error message

**Error Handling Methods**

Hyperswitch uses separate methods for different HTTP error types:

- **4xx Client Errors**: [`get_error_response`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L692) handles authentication failures, validation errors, and malformed requests.
- **5xx Server Errors**: 
[`get_5xx_error_response`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L700) handles internal server errors with potential retry logic.

Both methods delegate to [`build_error_response`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L136) for consistent processing.

**Error Processing Flow**

The [`build_error_response`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L136) struct serves as the intermediate data structure that bridges Billwerk's API error format and Hyperswitch's standardized error format by taking the `BillwerkErrorResponse` struct as input:

```rs
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
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
        })
    }
}
```
The method performs these key operations:

- Parses the HTTP response - Deserializes the raw HTTP response into a BillwerkErrorResponse struct using `parse_struct("BillwerkErrorResponse")`

- Logs the response - Records the connector response for debugging via `event_builder` and `router_env::logger::info!`

- Transforms error format - Maps Billwerk's error fields to Hyperswitch's standardized `ErrorResponse` structure with appropriate fallbacks:
- - Uses `response.code` maps to `code` (with `NO_ERROR_CODE fallback`)
- - Uses `response.message` maps to `message` (with `NO_ERROR_MESSAGE fallback`)
- -  Maps `response.error` to the `reason` field

> [!NOTE]
> When the connector provides only a single error message field, populate both the `message` and `reason` fields in the `ErrorResponse` with the same value. The `message` field is used for smart retries logic, while the `reason` field is displayed on the Hyperswitch dashboard.


### Automatic Error Routing

Hyperswitch's core API automatically routes errors based on HTTP status codes. You can find the details here: [`crates/router/src/services/api.rs`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/services/api.rs#L1).

- 4xx → [`get_error_response`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L256)
- 5xx → [`get_5xx_error_response`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L264)
- 2xx → [`handle_response`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L332)


### Integration Pattern

The `BillwerkErrorResponse` struct serves as the intermediate data structure that bridges Billwerk's API error format and Hyperswitch's internal error representation. The method essentially consumes the struct and produces Hyperswitch's standardized error format. All connectors implement a similar pattern to ensure uniform error handling. 

## Implementing the Connector Interface
The connector interface implementation follows an architectural pattern that separates concerns between data transformation and interface compliance.


- `transformers.rs` - This file is generated from `add_connector.sh` and defines the data structures and conversion logic for PSP-specific formats. This is where most of your custom connector implementation work happens.

- `mod.rs` - This file implements the standardized Hyperswitch connector interface using the transformers.

### The `mod.rs` Implementation Pattern
The file creates the bridge between the data transformation logic (defined in `transformers.rs`) and the connector interface requirements. It serves as the main connector implementation file that brings together all the components defined in the transformers module and implements all the required traits for payment processing. Looking at the connector template structure [`connector-template/mod.rs:54-67`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/connector-template/mod.rs#L54-L67), you can see how it:

- **Imports the transformers module** - Brings in your PSP-specific types and conversion logic
```rs
use transformers as {{project-name | downcase}};
```

- **Creates the main connector struct** - A struct named after your connector that holds the implementation
```rs
#[derive(Clone)]
pub struct {{project-name | downcase | pascal_case}} {
    amount_converter: &'static (dyn AmountConvertor<Output = StringMinorUnit> + Sync)
}

impl {{project-name | downcase | pascal_case}} {
    pub fn new() -> &'static Self {
        &Self {
            amount_converter: &StringMinorUnitForConnector
        }
    }
}
```

- **Implements required traits** - Provides the standardized methods Hyperswitch expects
```rs
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
            network_advice_code: None,
            network_decline_code: None,
            network_error_message: None,
        })
    }
}
```

## ConnectorCommon: The Foundation Trait
The `ConnectorCommon` trait defines the standardized interface required by Hyperswitch (as outlined in [`crates/hyperswitch_interfaces/src/api.rs`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_interfaces/src/api.rs#L326-L374) and acts as the bridge to your PSP-specific logic in `transformers.rs`. The `connector-template/mod.rs` file implements this trait using the data types and transformation functions from `transformers.rs`. This allows Hyperswitch to interact with your connector in a consistent, processor-agnostic manner. Every connector must implement the `ConnectorCommon` trait, which provides essential connector properties:

### Core Methods You'll Implement

- `id()` - Your connector's unique identifier
```rs 
fn id(&self) -> &'static str {
      "Billwerk"
  }
```

- `get_currency_unit()` - Whether you handle amounts in base units (dollars) or minor units (cents).
```rs
  fn get_currency_unit(&self) -> api::CurrencyUnit {
      api::CurrencyUnit::Minor
  }
  ```

- `base_url()` - This fetches your PSP's API endpoint
```rs
   fn base_url<'a>(&self, connectors: &'a Connectors) -> &'a str {
        connectors.billwerk.base_url.as_ref()
    }
```

- `get_auth_header()` - How to authenticate with your PSP
```rs
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

- `build_error_response()` - How to transform your PSP's errors into Hyperswitch's format
```rs
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

## `ConnectorIntegration` - The Payment Flow Orchestrator
The `ConnectorIntegration` trait serves as the central coordinator that bridges three key files in Hyperswitch's connector architecture:

- **Defined in `api.rs`**  
  [`crates/hyperswitch_interfaces/src/api.rs:150–153`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_interfaces/src/api.rs#L156-%23L159)  
  Provides the standardized interface contracts for connector integration.

- **Implemented in `mod.rs`**  
  Each connector’s main file (`mod.rs`) implements the trait methods for specific payment flows like authorize, capture, refund, etc. You can see how the Tsys connector implements [ConnectorIntegration](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/tsys.rs#L219)

- **Uses types from `transformers.rs`**  
  Contains PSP-specific request/response structs and `TryFrom` implementations that convert between Hyperswitch's internal `RouterData` format and the PSP's API format. This is where most connector-specific logic lives.

This orchestration enables seamless translation between Hyperswitch’s internal data structures and each payment service provider’s unique API requirements.

## Method-by-Method Breakdown

### Request/Response Flow  
These methods work together in sequence:  
1. `get_url()` and `get_headers()` prepare the endpoint and authentication  
2. `get_request_body()` transforms Hyperswitch data using transformers.rs  
3. `build_request()` assembles the complete HTTP request  
4. `handle_response()` processes the PSP response back to Hyperswitch format  
5. `get_error_response()` handles any error conditions

Here are more examples around these methods in the Billwerk connector:
- **`get_url()`**  
  Constructs API endpoints by combining base URLs (from `ConnectorCommon`) with specific paths. In the Billwerk connector, it reads the connector’s base URL from config and appends the tokenization path. [Here's 1 example](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L193-L204).

- **`get_headers()`**  
  Here's an example of [get_headers](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L618). It delegates to [`build_headers()`](https://github.com/juspay/hyperswitch/blob/06dc66c62e33c1c56c42aab18a7959e1648d6fae/crates/hyperswitch_interfaces/src/api.rs#L422-L430) across all connector implementations. 

- **`get_request_body()`**  
  Uses the `TryFrom` implementations in [billwerk.rs](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L206-L213). It creates the connector request via  `BillwerkTokenRequest::try_from(req)?` to transform the tokenization router data and it 
returns as `RequestContent:` by wrapping it in a JSON via `RequestContent::Json(Box::new(connector_req))`  

- **`build_request()`**  
  Orchestrates `get_url()`, `get_headers()`, and `get_request_body()` to assemble the complete HTTP request via a `RequestBuilder`. For example, you can review the Billwerk connector's [`build_request()`](https://github.com/juspay/hyperswitch/blob/b133c534fb1ce40bd6cca27fac4f2d58b0863e30/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L215-L231) implementation. 

- **`handle_response()`**  
  You can see an example of this here: [`billwerk.rs`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L332). In this example, it parses the raw response into `BillwerkTokenResponse` using `res.response.parse_struct()`, logs the response with an `event_builder.map(|i| i.set_response_body(&response))`, finally it 
transforms back to `RouterData` using `RouterData::try_from(ResponseRouterData {...}) `.

- **`get_error_response()`**
  Here's an example of [get_error_response](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L256) in `billewerk.rs`. It delegates to [`build_error_response()`](https://github.com/juspay/hyperswitch/blob/b133c534fb1ce40bd6cca27fac4f2d58b0863e30/crates/hyperswitch_connectors/src/connectors/billwerk.rs#L136-L162) from the `ConnectorCommon` trait, providing uniform handling for all connector 4xx errors.  


### `ConnectorCommonExt` - Generic Helper Methods
The [`ConnectorCommonExt`](https://github.com/juspay/hyperswitch/blob/06dc66c6/crates/hyperswitch_interfaces/src/api.rs#L418-L441) trait serves as an extension layer for the core `ConnectorCommon` trait, providing generic methods that work across different payment flows. It'requires both ConnectorCommon and ConnectorIntegration to be implemented.

## Connector Traits Overview

### `Payment`  
Includes several sub-traits and represents general payment functionality.  
- **Defined in:** [`crates/hyperswitch_interfaces/src/types.rs:11-16`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_interfaces/src/types.rs#L11-L16)  
- **Example implementation:** [`crates/hyperswitch_connectors/src/connectors/novalnet.rs:70`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/novalnet.rs#L70)  

### `PaymentAuthorize`  
Extends the `api::ConnectorIntegration` trait with types for payment authorization.  
- **Flow type defined in:** [`crates/router/src/types.rs:39`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L39)  
- **Example implementation:** [`crates/hyperswitch_connectors/src/connectors/novalnet.rs:74`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/novalnet.rs#L74)  

### `PaymentCapture`  
Extends the `api::ConnectorIntegration` trait with types for manual capture of a previously authorized payment.  
- **Flow type defined in:** [`crates/router/src/types.rs:39`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L39)  
- **Example implementation:** [`crates/hyperswitch_connectors/src/connectors/novalnet.rs:76`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/novalnet.rs#L76)  

### `PaymentSync`  
Extends the `api::ConnectorIntegration` trait with types for retrieving or synchronizing payment status.  
- **Flow type defined in:** [`crates/router/src/types.rs:41`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L41)  
- **Example implementation:** [`crates/hyperswitch_connectors/src/connectors/novalnet.rs:75`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/novalnet.rs#L75)  

### `Refund`  
Includes several sub-traits and represents general refund functionality.  
- **Defined in:** [`crates/hyperswitch_interfaces/src/types.rs:17`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_interfaces/src/types.rs#L17)  
- **Example implementation:** [`crates/hyperswitch_connectors/src/connectors/novalnet.rs:78`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/novalnet.rs#L78)  

### `RefundExecute`  
Extends the `api::ConnectorIntegration` trait with types for creating a refund.  
- **Flow type defined in:** [`crates/router/src/types.rs:44`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L44)  
- **Example implementation:** [`crates/hyperswitch_connectors/src/connectors/novalnet.rs:79`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/novalnet.rs#L79)  

### `RefundSync`  
Extends the `api::ConnectorIntegration` trait with types for retrieving or synchronizing a refund.  
- **Flow type defined in:** [`crates/router/src/types.rs:44`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/types.rs#L44)  
- **Example implementation:** [`crates/hyperswitch_connectors/src/connectors/novalnet.rs:80`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/novalnet.rs#L80) 

## Connector Required Fields Configuration

The file [`crates/payment_methods/src/configs/payment_connector_required_fields.rs`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/payment_methods/src/configs/payment_connector_required_fields.rs#L1) is the central configuration file that defines required fields for each connector and payment-method combination.

### Example: Billwerk Required Fields

Based on the required-fields configuration, Billwerk requires only basic card details for card payments. Please see [`payment_connector_required_fields.rs:1271`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/payment_methods/src/configs/payment_connector_required_fields.rs#L1271).

Specifically, Billwerk requires:
- Card number  
- Card expiry month  
- Card expiry year  
- Card CVC  

This is defined using the `card_basic()` helper (see [`payment_connector_required_fields.rs:876–884`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/payment_methods/src/configs/payment_connector_required_fields.rs#L876-L884)), which specifies these four essential card fields as `RequiredField` enum variants.

### Comparison with Other Connectors

Billwerk has relatively minimal requirements compared to other connectors. For example:

- **Bank of America** requires card details plus email, full name, and complete billing address (see [`payment_connector_required_fields.rs:1256–1262`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/payment_methods/src/configs/payment_connector_required_fields.rs#L1256-L1262)).  
- **Cybersource** requires card details, billing email, full name, and billing address (see [`payment_connector_required_fields.rs:1288–1294`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/payment_methods/src/configs/payment_connector_required_fields.rs#L1288-L1294)).

Please review the file for your specific connector requirements.

## Derive Traits

The derive traits are standard Rust traits that are automatically implemented:

- **Debug**: Standard Rust trait for debug formatting. It's automatically derived on connector structs like [`crates/hyperswitch_connectors/src/connectors/coinbase.rs:52`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/coinbase.rs#L52)
- **Clone**: Standard Rust trait for cloning. It's implemented on connector structs like [`crates/hyperswitch_connectors/src/connectors/novalnet.rs:57`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/connectors/novalnet.rs#L58)
- **Copy**: Standard Rust trait for copy semantics. It's used where applicable for simple data structures

These traits work together to provide a complete payment processing interface, with each trait extending `ConnectorIntegration` with specific type parameters for different operations.

## Connector utility functions
Hyperswitch provides a set of standardized utility functions to streamline data extraction, validation, and formatting across all payment connectors. These are primarily defined in:

- [`crates/hyperswitch_connectors/src/utils.rs`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/hyperswitch_connectors/src/utils.rs#L1)
- [`crates/router/src/connector/utils.rs`](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/router/src/connector/utils.rs#L1)

###  Key Utilities and Traits

#### `RouterData` Trait  
Provides helper methods to extract billing and browser data:

- `get_billing_country()` – Retrieves the billing country  
- `get_billing_email()` – Gets the customer email from billing data  
- `get_billing_full_name()` – Extracts full name  
- `get_browser_info()` – Parses browser details for 3DS  
- `is_three_ds()` – Checks if 3DS is required  
- `is_auto_capture()` – Determines if auto-capture is enabled  

---

#### `CardData` Trait  
Handles card-specific formatting and parsing:

- `get_expiry_date_as_yyyymm()` – Formats expiry as YYYYMM  
- `get_expiry_date_as_mmyyyy()` – Formats expiry as MMYYYY  
- `get_card_expiry_year_2_digit()` – Gets 2-digit expiry year  
- `get_card_issuer()` – Returns card brand (Visa, Mastercard, etc.)  
- `get_cardholder_name()` – Extracts name on card  

---

#### Wallet Data
Utility for processing digital wallet tokens:

```rs
let json_wallet_data: CheckoutGooglePayData = wallet_data.get_wallet_token_as_json()?; 
```
### Real-World Usage Examples
- PayPal Connector: `get_expiry_date_as_yyyymm()` is used for tokenization and authorization

- Bambora Connector: `get_browser_info()` is used to enables 3DS and `is_auto_capture()` is used to check capture behavior

- Trustpay Connector: Uses extensive browser info usage for 3DS validation flows


### Error Handling & Validation
- `missing_field_err()` – Commonly used across connectors for standardized error reporting

## Connector Configuration for Control Center Integration
This guide helps developers integrate custom connectors with the Hyperswitch Control Center by configuring connector settings and building the required WebAssembly components.

## Prerequisites

Install the WebAssembly build tool:

```bash
cargo install wasm-pack
```
### Step 1: Configure Your Connector
Add your connector configuration to the [development environment file](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/connector_configs/toml/development.toml)

The connector configuration system does support multiple environments as you mentioned. The system automatically selects the appropriate configuration file based on feature flags:

- Production: [crates/connector_configs/toml/production.toml](https://github.com/juspay/hyperswitch/blob/06dc66c62e33c1c56c42aab18a7959e1648d6fae/crates/connector_configs/toml/production.toml)
- Sandbox: [crates/connector_configs/toml/sandbox.toml](https://github.com/juspay/hyperswitch/blob/06dc66c62e33c1c56c42aab18a7959e1648d6fae/crates/connector_configs/toml/sandbox.toml)
- Development: [crates/connector_configs/toml/development.toml (default)](https://github.com/juspay/hyperswitch/blob/06dc66c62e33c1c56c42aab18a7959e1648d6fae/crates/connector_configs/toml/development.toml)

```rs
# Example: Adding a new connector configuration
[your_connector_name]
[your_connector_name.connector_auth.HeaderKey]
api_key = "Your_API_Key_Here"

# Optional: Add additional connector-specific settings
[your_connector_name.connector_webhook_details]
merchant_secret = "webhook_secret"
```
### Step 2: Build WebAssembly Components
The Control Center requires WebAssembly files for connector integration. Build them using:

```bash
wasm-pack build \
  --target web \
  --out-dir /path/to/hyperswitch-control-center/public/hyperswitch/wasm \
  --out-name euclid \
  /path/to/hyperswitch/crates/euclid_wasm \
  -- --features dummy_connector
```
- Replace `/path/to/hyperswitch-control-center` with your Control Center installation directory

- Replace `/path/to/hyperswitch` with your Hyperswitch repository root

The build process uses the [`euclid_wasm` crate](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/euclid_wasm/Cargo.toml#L1-L44), which provides WebAssembly bindings for connector configuration and routing logic.

### Step 3: Verify Integration
The WebAssembly build includes [connector configuration functions](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/crates/euclid_wasm/src/lib.rs#L376-L382) that the Control Center uses to retrieve connector settings dynamically.

You can also use the Makefile target for convenience:

```bash
make euclid-wasm
```

This target is defined in the [Makefile:86-87](https://github.com/juspay/hyperswitch/blob/2309c5311cb9a01ef371f3a3ef7c62c88a043696/Makefile#L86-L87) and handles the build process with appropriate feature flags.

### Configuration Features
The connector configuration system supports:

- **Environment-specific configs**: [Development, sandbox, and production configurations](https://github.com/juspay/hyperswitch/blob/06dc66c6/crates/connector_configs/src/connector.rs#L323-L337)

- **Authentication methods**: HeaderKey, BodyKey, SignatureKey, etc.

- **Webhook configuration**: For handling asynchronous payment notifications

- **Payment method support**: Defining which payment methods your connector supports

### Troubleshooting
If the build fails, ensure:

- Your connector is properly registered in the connector enum
- **The WebAssembly target is installed: `rustup target add wasm32-unknown-unknown`**
- All required features are enabled in your connector's `Cargo.toml`
- The configuration system automatically loads the appropriate environment settings based on compile-time features, ensuring your connector works correctly across different deployment environments.

## Control Center Frontend Integration
This section covers integrating your new connector with the Hyperswitch Control Center's frontend interface, enabling merchants to configure and manage your connector through the dashboard.

### Update Frontend Connector Configuration
1. Add Connector to Type Definitions

Update the connector enum in the [Control Center's type definitions](https://github.com/juspay/hyperswitch-control-center/blob/e984254b68511728b6b37890fd0c7c7e90c22f57/src/screens/Connectors/ConnectorTypes.res#L29)

```rs
type processorTypes =  
  | BREADPAY
  | BLUECODE 
  | YourNewConnector  // Add your connector here at the bottom
```
### Update Connector Utilities
Modify the [connector utilities](https://github.com/juspay/hyperswitch-control-center/blob/e984254b68511728b6b37890fd0c7c7e90c22f57/src/screens/Connectors/ConnectorUtils.res#L46) to include your new connector.

```js
// Add to connector list at the bottom 
let connectorList: array<connectorTypes> = [
....
  Processors(BREADPAY),
  Processors(BLUECODE),
  Processors(YourNewConnector)
]  
  
// Add display name mapping at the bottom 
let getConnectorNameString = (connectorName: connectorName) =>  
  switch connectorName {  
  | BREADPAY => "breadpay"
  | BLUECODE => "bluecode" 
  | YourNewConnector => "Your New Connector"  
  }  
  
// Add connector description at the bottom 
let getProcessorInfo = (connector: ConnectorTypes.processorTypes) => { 
  switch connectorName {  
  | BREADPAY => breadpayInfo
  | BLUECODE => bluecodeInfo
  | YourNewConnector => YourNewConnectorInfo  
  }
```
After [`bluecodeinfo`](https://github.com/juspay/hyperswitch-control-center/blob/e984254b68511728b6b37890fd0c7c7e90c22f57/src/screens/Connectors/ConnectorUtils.res#L693) definition, add the definition of your connector in a similar format: 

```js
let YourNewConnectorInfo = {
  description: "Info for the connector.",
}
```

### Add Connector Icon
1. Prepare Icon Asset
- Create an SVG icon for your connector
- Name it in uppercase format: YOURCONNECTOR.SVG
- Ensure the icon follows the design guidelines (typically 24x24px or 32x32px)
2. Add to Assets Directory
Place your icon in the Control Center's gateway assets folder:

```text
public/  
└── hyperswitch/  
    └── Gateway/  
        └── YOURCONNECTOR.SVG
```
The icon will be automatically loaded by the frontend based on the connector name mapping.

---
## Test the Connector Integration
After successfully creating your connector using the `add_connector.sh` script, you need to configure authentication credentials and test the integration. This section covers the complete testing setup process.

### Authentication Setup
1. **Obtain PSP Credentials**

First, obtain sandbox/UAT API credentials from your payment service provider. These are typically available through their developer portal or dashboard.

2. **Create Authentication File**

Copy the sample authentication template and create your credentials file:

```bash 
cp crates/router/tests/connectors/sample_auth.toml auth.toml
```

The sample file `crates/router/tests/connectors/sample_auth.toml` contains templates for all supported connectors. Edit your `auth.toml` file to include your connector's credentials:

**Example for the Billwerk connector**  

```text
[billewerk]  
api_key = "sk_test_your_actual_billwerk_test_key_here"  
```

3. **Configure Environment Variables**

Set the path to your authentication file:

```bash
export CONNECTOR_AUTH_FILE_PATH="/absolute/path/to/your/auth.toml"
```

4. **Use `direnv` for Environment Management (recommended)**

For better environment variable management, use `direnv` with a `.envrc` file in the `cypress-tests` directory.

5. **Create `.envrc` in the `cypress-tests` directory**

```bash
cd cypress-tests
```
**Create a `.envrc` file with the following content**:

```bash
export CONNECTOR_AUTH_FILE_PATH="/absolute/path/to/your/auth.toml"
export CYPRESS_CONNECTOR="your_connector_name"
export CYPRESS_BASEURL="http://localhost:8080"
export CYPRESS_ADMINAPIKEY="test_admin"
export DEBUG=cypress:cli
```

6. **Allow `direnv` to load the variables inside the `cypress-tests` directory**: 

``` bash
direnv allow
```

### Test the Connector Integration

1. **Start the Hyperswitch Router Service locally**:

```bash
cargo r
```

2. **Verify Server Health**

```bash 
curl --head --request GET 'http://localhost:8080/health'
```
  
**Detailed health check**

```bash
curl --request GET 'http://localhost:8080/health/ready'
```

3. **Run Connector Tests for Your Connector**

```bash
cargo test --package router --test connectors -- your_connector_name --test-threads=1
```

The authentication system will load your credentials from the specified path and use them for testing.

> **⚠️ Important Notes**
>
> * **Never commit `auth.toml`** – It contains sensitive credentials and should never be added to version control
> * **Use absolute paths** – This avoids issues when running tests from different directories
> * **Populate with real test credentials** – Replace the placeholder values from the sample file with actual sandbox/UAT credentials from your payment processors. Please don't use production credentials. 
> * **Rotate credentials regularly** – Update test keys periodically for security.