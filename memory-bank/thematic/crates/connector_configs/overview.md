# Connector Configs Overview

The `connector_configs` crate manages payment connector configurations and settings for Hyperswitch. This document provides an overview of its purpose, structure, and usage within the Hyperswitch ecosystem.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `connector_configs` crate is responsible for:

1. Defining data structures for connector configurations
2. Loading and parsing connector configurations from TOML files
3. Providing access to connector-specific settings and authentication details
4. Supporting different environment configurations (development, sandbox, production)
5. Managing payment method configurations for different connectors

## Key Modules

The `connector_configs` crate is organized into the following key modules:

- **connector.rs**: Defines the primary connector configuration structures and methods to retrieve connector-specific configurations
- **common_config.rs**: Contains shared configuration structures used across different connectors
- **response_modifier.rs**: Handles modification of connector responses based on configuration
- **transformer.rs**: Provides functionality to transform data between Hyperswitch's internal format and connector-specific formats

## Core Features

### Configuration Management

The crate loads connector configurations from TOML files based on the current environment:

- Development configuration (default)
- Sandbox configuration (when `sandbox` feature is enabled)
- Production configuration (when `production` feature is enabled)

All configurations are defined in the `toml/` directory with separate files for each environment.

### Connector Authentication

Supports multiple authentication methods for connectors:

- **HeaderKey**: Simple API key in the header
- **BodyKey**: API key in the request body
- **SignatureKey**: Authentication with API key, secret, and signature
- **MultiAuthKey**: Advanced authentication with multiple keys
- **CurrencyAuthKey**: Currency-specific authentication details
- **CertificateAuth**: Certificate-based authentication
- **NoKey**: No authentication required

Example of connector authentication configuration:

```rust
pub enum ConnectorAuthType {
    HeaderKey {
        api_key: String,
    },
    BodyKey {
        api_key: String,
        key1: String,
    },
    SignatureKey {
        api_key: String,
        key1: String,
        api_secret: String,
    },
    // Other auth types...
}
```

### Payment Method Configuration

Supports configuration for various payment methods across different connectors:

- Credit and debit cards
- Bank transfers and redirects
- Digital wallets
- Pay later services
- Cryptocurrencies
- UPI, vouchers, and gift cards

Each payment method can have connector-specific settings and requirements defined in the configuration.

### Connector Type Support

The crate provides configuration access for different types of connectors:

- Payment connectors (standard payment processing)
- Payout connectors (for disbursement operations)
- Authentication connectors (3DS and other authentication services)
- Tax connectors (tax calculation and management)
- PM authentication connectors (for payment method authentication)

## Public Interface

### Key Structs

```rust
// Main configuration structure for connectors
pub struct ConnectorTomlConfig {
    pub connector_auth: Option<ConnectorAuthType>,
    pub connector_webhook_details: Option<api_models::admin::MerchantConnectorWebhookDetails>,
    pub metadata: Option<Box<ConfigMetadata>>,
    pub connector_wallets_details: Option<Box<ConnectorWalletDetailsConfig>>,
    pub additional_merchant_data: Option<Box<ConfigMerchantAdditionalDetails>>,
    // Payment method configurations
    pub credit: Option<Vec<CardProvider>>,
    pub debit: Option<Vec<CardProvider>>,
    pub bank_transfer: Option<Vec<Provider>>,
    // Other payment methods...
}

// Container for all connector configurations
pub struct ConnectorConfig {
    pub adyen: Option<ConnectorTomlConfig>,
    pub stripe: Option<ConnectorTomlConfig>,
    pub checkout: Option<ConnectorTomlConfig>,
    // Many other connectors...
}
```

### Main Functions

```rust
// Get configuration for a specific payment connector
pub fn get_connector_config(
    connector: Connector,
) -> Result<Option<ConnectorTomlConfig>, String> {
    // Implementation details not included in documentation
}

// Get configuration for a payout connector
#[cfg(feature = "payouts")]
pub fn get_payout_connector_config(
    connector: PayoutConnectors,
) -> Result<Option<ConnectorTomlConfig>, String> {
    // Implementation details not included in documentation
}

// Get configuration for an authentication connector
pub fn get_authentication_connector_config(
    connector: AuthenticationConnectors,
) -> Result<Option<ConnectorTomlConfig>, String> {
    // Implementation details not included in documentation
}

// Other getter functions for different connector types...
```

## Usage Examples

### Retrieving Connector Configuration

```rust
use connector_configs::connector::ConnectorConfig;
use api_models::enums::Connector;

fn get_stripe_config() -> Result<Option<ConnectorTomlConfig>, String> {
    // Get Stripe connector configuration
    let stripe_config = ConnectorConfig::get_connector_config(Connector::Stripe)?;
    
    // Now you can access stripe-specific settings
    if let Some(config) = stripe_config {
        // Access API keys, webhook settings, etc.
        if let Some(auth) = config.connector_auth {
            // Use auth details for requests
        }
    }
    
    Ok(stripe_config)
}
```

### Working with Connector Authentication

```rust
use connector_configs::connector::{ConnectorConfig, ConnectorAuthType};
use api_models::enums::Connector;

fn prepare_auth_headers(connector: Connector) -> Result<HashMap<String, String>, String> {
    let mut headers = HashMap::new();
    
    // Get connector configuration
    if let Some(config) = ConnectorConfig::get_connector_config(connector)? {
        // Extract authentication details
        if let Some(auth) = config.connector_auth {
            match auth {
                ConnectorAuthType::HeaderKey { api_key } => {
                    headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));
                },
                // Handle other auth types...
                _ => {}
            }
        }
    }
    
    Ok(headers)
}
```

## Integration with Other Crates

The `connector_configs` crate integrates with several other parts of the Hyperswitch ecosystem:

1. **api_models**: Uses enumerations and data structures from api_models for connector types and configuration details
2. **hyperswitch_connectors**: Provides configuration information used by connectors when making API calls
3. **router**: The router crate uses connector configurations when routing payments to appropriate connectors
4. **common_utils**: Utilizes common utilities for parsing and handling configuration data

## Configuration Options

The crate offers several feature flags to control its behavior:

- **default**: Includes "payouts" and "dummy_connector" features
- **production**: Enables production configuration loading
- **sandbox**: Enables sandbox configuration loading
- **dummy_connector**: Enables support for test/dummy connectors
- **payouts**: Enables support for payout connectors
- **v1**: Compatibility with v1 API models

## Error Handling

Configuration errors are returned as string error messages. The primary error scenarios include:

- Configuration file parsing errors
- Missing connector configurations
- Attempting to access connector configurations using incorrect accessor methods

## Performance Considerations

- Configurations are loaded on-demand rather than pre-loaded to minimize memory usage
- TOML files are included at compile time for efficiency using `include_str!`

## Thread Safety and Async Support

The crate provides thread-safe access to configurations:

- All configuration structures implement `Clone` to avoid sharing mutable state
- No global mutable state is maintained
- Configuration retrieval methods are not async but can be safely called from async contexts

## Conclusion

The `connector_configs` crate serves as the central configuration management system for all payment connectors in the Hyperswitch ecosystem. It provides a flexible and type-safe way to access connector-specific settings, authentication details, and payment method configurations, ensuring that the system can correctly communicate with various payment service providers.

## See Also

- [Hyperswitch Connectors Overview](../hyperswitch_connectors/overview.md)
- [Router Crate Overview](../router/overview.md)
- [API Models Overview](../api_models/overview.md)
