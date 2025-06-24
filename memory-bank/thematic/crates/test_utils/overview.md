# Test Utilities (test_utils) Overview

The `test_utils` crate provides testing utilities for the Hyperswitch project, with a primary focus on running Postman collections using the Newman runner. This document outlines its purpose, components, and usage within the Hyperswitch ecosystem.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `test_utils` crate is responsible for:

1. Running Postman collections to test connector integrations
2. Managing authentication for connector tests
3. Configuring Newman runner for different test scenarios
4. Supporting user-related tests
5. Handling custom headers and environment variables for tests
6. Modifying collections at runtime to handle special cases

## Key Modules

The `test_utils` crate is organized into the following modules:

- **connector_auth.rs**: Manages authentication configuration for different payment connectors
- **newman_runner.rs**: Provides functionality for running Postman collections using Newman
- **main.rs**: Entry point for the CLI tool
- **lib.rs**: Exposes the functionality as a library for use by other crates

## Core Features

### Postman Collection Runner

The crate provides a Newman-based runner for Postman collections:

- **Command Generation**: Building properly configured Newman commands
- **Environment Variables**: Setting up test-specific environment variables
- **Collection Handling**: Managing collection paths and directory structures
- **Module-Specific Commands**: Specialized commands for different test modules (connectors, users)

### Connector Authentication

Support for various authentication types across different payment connectors:

- **Authentication Types**: Support for HeaderKey, BodyKey, SignatureKey, and MultiAuthKey
- **Secret Management**: Secure handling of API keys and secrets
- **Authentication Mapping**: Associating connectors with their required authentication details

### Runtime Collection Modification

Capabilities for modifying collections at runtime to handle special cases:

- **Dynamic Amount Handling**: Special handling for connectors requiring integer amount values
- **Custom Header Injection**: Adding custom headers to requests at runtime
- **Collection Export**: Converting between directory and JSON collection formats

### CLI Interface

A comprehensive command-line interface for running tests:

- **Command Arguments**: Configurable command-line arguments for test runs
- **Module Selection**: Support for running tests for specific modules
- **Folder Selection**: Running specific test folders within collections
- **Verbosity Control**: Optional verbose logging for detailed output

## Public Interface

### Key Types and Functions

```rust
// Types
pub enum Module {
    Connector,
    Users,
}

pub struct Args {
    // Command-line arguments for the runner
}

pub struct ReturnArgs {
    pub newman_command: Command,
    pub modified_file_paths: Vec<Option<String>>,
    pub collection_path: String,
}

// Functions
pub fn generate_runner() -> Result<ReturnArgs>
pub fn generate_newman_command_for_connector() -> Result<ReturnArgs>
pub fn generate_newman_command_for_users() -> Result<ReturnArgs>
```

## Usage Examples

### Running Connector Tests

```rust
use test_utils::newman_runner;

fn main() -> anyhow::Result<()> {
    // Generate a Newman runner command for a connector
    let return_args = newman_runner::generate_runner()?;
    
    // Access the command and execute it
    let mut newman_command = return_args.newman_command;
    let status = newman_command.spawn()?.wait()?;
    
    if status.success() {
        println!("Tests completed successfully!");
    } else {
        eprintln!("Tests failed with exit code: {:?}", status.code());
    }
    
    Ok(())
}
```

### Command-Line Usage

```bash
# Run tests for a specific connector
test_utils --admin-api-key <KEY> --base-url <URL> --connector-name stripe

# Run user module tests
test_utils --admin-api-key <KEY> --base-url <URL> --module-name users

# Run specific test folders with custom headers
test_utils --admin-api-key <KEY> --base-url <URL> --connector-name stripe \
  --folder "QuickStart,Payment" \
  --header "X-Custom-Header:value" \
  --verbose
```

## Authentication Configuration

The crate expects authentication details for connectors in a TOML configuration file, specified by the `CONNECTOR_AUTH_FILE_PATH` environment variable. The file structure should follow this format:

```toml
[connector_auth.stripe]
auth_type = "header_key"
api_key = "sk_test_..."

[connector_auth.adyen]
auth_type = "signature_key"
api_key = "your_api_key"
key1 = "your_merchant_account"
api_secret = "your_api_secret"

[users]
user_email = "test@example.com"
user_base_email_for_signup = "test"
user_domain_for_signup = "@example.com"
user_password = "Password123"
wrong_password = "WrongPassword"
```

## Integration with Other Crates

The `test_utils` crate is designed to be lightweight with minimal dependencies:

1. **masking**: Used for secure handling of sensitive information
2. **clap**: Used for command-line argument parsing
3. **anyhow**: Used for error handling
4. **reqwest**: Used for HTTP client functionality
5. **serde** and **toml**: Used for configuration parsing

## Feature Flags

The crate supports several feature flags:

- **default**: Enables both `dummy_connector` and `payouts`
- **dummy_connector**: Enables support for the dummy connector for testing
- **payouts**: Enables payout-related functionality

## Special Handling for Connectors

Some connectors require special handling:

- **Dynamic Amount Connectors**: Connectors like NMI and PowerTranz require integer values for amounts without quotes
- **Certificate-Requiring Connectors**: Support for connectors requiring certificates through environment variables
- **Custom Header Connectors**: Support for connectors requiring custom headers

## Implementation Notes

### Collection Structure

The crate expects collections to be organized in a specific structure:

- **JSON Collections**: Located at `postman/collection-json/<connector>.postman_collection.json`
- **Directory Collections**: Located at `postman/collection-dir/<connector>`

### Environment Variables

The following environment variables can be used to configure the runner:

- **CONNECTOR_AUTH_FILE_PATH**: Path to the connector authentication configuration file
- **GATEWAY_MERCHANT_ID**: Optional merchant ID for specific gateways
- **GPAY_CERTIFICATE** and **GPAY_CERTIFICATE_KEYS**: For Google Pay-related tests

## Testing Strategy

When using this crate for testing:

1. **Setup**: Ensure proper authentication configuration is available
2. **Execution**: Run collections with appropriate parameters
3. **Validation**: Verify test results through Newman output
4. **Clean-up**: Handle any test artifacts or state

## Conclusion

The `test_utils` crate provides a robust framework for testing Hyperswitch's connector integrations using Postman collections. It abstracts away the complexity of running Newman commands, handling authentication, and managing test environments, making it easier to ensure reliable testing of payment integrations.

While primarily focused on connector tests, the crate also supports user-related tests and can be extended to support other test modules as needed. Its modular design and configurable interface make it a valuable tool in Hyperswitch's testing strategy.
