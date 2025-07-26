# Guide to Integrating a Connector

## Table of Contents

1. [Introduction](#introduction)
2. [Prerequisites](#prerequisites)
3. [Development Environment Setup & Configuration](#development-environment-setup--configuration)

## Introduction

This guide provides instructions on integrating a new connector with Router, from setting up the environment to implementing API interactions. In this document you’ll learn how to:

* Scaffold a new connector template
* Define Rust request/response types directly from your PSP’s JSON schema
* Implement transformers and the ConnectorIntegration trait for both standard auth and tokenization-first flows
* Enforce PII best practices (Secret wrappers, common\_utils::pii types) and robust error-handling
* Update the Control-Center (ConnectorTypes.res, ConnectorUtils.res, icons)
* Validate your connector with end-to-end tests

By the end, you’ll have a fully functional, production-ready connector—from blank slate to live in the Control-Center.

## Prerequisites

* Before you begin, ensure you’ve completed the initial setup in our [Hyperswitch Contributor Guide](https://github.com/juspay/hyperswitch/blob/main/CONTRIBUTING.md), which covers cloning, tool installation, and access.
* You should also understanding [connectors and payment methods](https://hyperswitch.io/pm-list).
* Familiarity with the Connector API you’re integrating
* A locally set up and running Router repository
* API credentials for testing (sign up for sandbox/UAT credentials on the connector’s website).
* Need help? Join the [Hyperswitch Slack Channel](https://join.slack.com/t/hyperswitch-io/shared_invite/zt-39d4w0043-CgAyb75Kn0YldNyZpd8hWA). We also have weekly office hours every Thursday at 8:00 AM PT (11:00 AM ET, 4:00 PM BST, 5:00 PM CEST, and 8:30 PM IST). Link to office hours are shared in the #general channel.

## Development Environment Setup & Configuration

This guide will walk you through your environment setup and configuration.

### Clone the Hyperswitch monorepo\*\*

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
* [Set up database](https://github.com/juspay/hyperswitch/blob/main/docs/try_local_system.md#set-up-the-database)

* Set up the Rust nightly toolchain installed for code formatting:

```bash
rustup toolchain install nightly
```

* Install [Protobuf](https://protobuf.dev/installation/)

If you've completed the setup, you should now have:

* ✅ Rust & Cargo
* ✅ PostgreSQL (with a user and database created)
* ✅ Redis
* ✅ `diesel_cli`
* ✅ The `just` command runner
* ✅ Database migrations applied
* ✅ Set up the Rust nightly toolchain
* ✅ Installed Protobuf

You're ready to run Hyperswitch:

```bash
cargo run
```
## Create a Connector
From the root of the project, generate a new connector by running the following command. Use a single-word name for your connector:

```bash
sh scripts/add_connector.sh <connector_name> <connector_base_url>
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

## Test the connection 
Once you've successfully created your connector using the `add_connector.sh` script, you can verify the integration by starting the Hyperswitch router service:

```bash
cargo r
```

This launches the router application locally on port 8080, providing access to the complete Hyperswitch API. You can now test your connector implementation by making HTTP requests to the payment endpoints for operations like:

- Payment authorization and capture
- Payment synchronization
- Refund processing
- Webhook handling

Once your connector logic is implemented, this environment lets you ensure it behaves correctly within the Hyperswitch orchestration flow—before moving to staging or production. This provides comprehensive status information about all system components including database, Redis, and other services.

**Verify Server Health**

Once the router service is running, `cargo r`, you can verify it's operational by checking the health endpoint in a separte terminal window:

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
add_connector.md:62-71

**Test Files**

The script also generates test files in the router crate:

crates/router/tests/connectors/  
└── <connector_name>.rs 

**What Each File Contains**

- `<connector_name>.rs`: The main connector implementation file where you implement the connector traits
- `transformers.rs`: Contains data structures and conversion logic between Hyperswitch's internal format and your payment processor's API format
- **Test file**: Contains boilerplate test cases for your connector connector-template/test.rs:1-36

## Common Payment Flow Types

As you build your connector, you'll encounter several types of payment flows. While not an exhaustive list, the following are some of the most common patterns you'll come across. Please review the [Connector Payment Flow](#) documentation for more details.

## Integrate a New Connector

Integrating a connector is mainly an API integration task. You'll define request and response types and implement required traits.

This section covers card payments via Billwerk. Review the API reference and test APIs before starting.

### Build Payment Request and Response from JSON Schema

1. **To generate Rust types from your connector’s OpenAPI or JSON schema, you’ll need to install the [OpenAPI Generator](https://openapi-generator.tech/).**

**Example (macOS using Homebrew)**:
```bash
brew install openapi-generator
```
> 💡 **Note:**  
> On **Linux**, you can install OpenAPI Generator using `apt`, `snap`, or by downloading the JAR from the [official site](https://openapi-generator.tech/docs/installation).  
> On **Windows**, use [Scoop](https://scoop.sh/) or manually download the JAR file.

2. **Download the OpenAPI Specification**

**Download the Connector's OpenAPI Schema**

First, obtain the OpenAPI specification from your payment processor's developer documentation. Most processors provide these specifications at standardized endpoints.

```bash
curl -o <connector-name>-openapi.json <schema-url>
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

`crates/hyperswitch_connectors/src/connectors/{CONNECTOR_NAME}/temp.rs `

This single file contains all the Rust structs and types generated from your payment processor's OpenAPI specification.

The generated `temp.rs` file typically contains:

- **Request structs**: Data structures for API requests
- **Response structs**: Data structures for API responses
- **Enum types**: Status codes, payment methods, error types
- **Nested objects**: Complex data structures used within requests/responses
- **Serde annotations**: Serialization/deserialization attributes.

Otherwise, you can manually define it and create the `crates/hyperswitch_connectors/src/connectors/{CONNECTOR_NAME}/temp.rs ` file. We highly recommend using the openapi-generator for ease. 

**Usage in Connector Development**

You can then copy and adapt these generated structs into your connector's `transformers.rs` file, following the pattern shown in the connector integration documentation. The generated code serves as a starting point that you customize for your specific connector implementation needs.

3. **Configure Required Environment Variables**

Set up the necessary environment variables for the OpenAPI generation process:

#### Connector name (must match the name used in add_connector.sh script) 

```bash
export CONNECTOR_NAME="your_connector_name"  
``` 

#### Path to the downloaded OpenAPI specification  
```bash
export SCHEMA_PATH="/absolute/path/to/your/connector-openapi.json"
```

## Code Walkthrough

We'll walk through the `transformer.rs` file, and what needs to be implemented.

1. **Converts Hyperswitch's internal payment data into your connector's API request format**
 This part of the code takes your internal representation of a payment request, pulls out the token, gathers all the customer and payment fields, and packages them into a clean, JSON-serializable struct ready to send to Billwerk. You'll have to implement the customer and payment fields, as necessary, but you can implement it below: 

```rust
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

Implement From<ConnectorStatus> for Hyperswitch’s `AttemptStatus` enum:

```rs
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










--


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