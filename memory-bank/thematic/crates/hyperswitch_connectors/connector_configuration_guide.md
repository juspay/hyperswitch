---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

# Connector Configuration Guide

---
**Parent:** [Hyperswitch Connectors Overview](./overview.md)  
**Related Files:**
- [Connector Interface Requirements](./connector_interface_guide.md)
- [Connector Implementation Guide](./connector_implementation_guide.md)
- [Connector Testing Guide](./connector_testing_guide.md)
---

## Overview

This guide provides detailed information about configuring payment connectors in Hyperswitch. It covers configuration file structure, environment variables, and runtime configuration options. Proper configuration is essential for connectors to function correctly and securely.

## Configuration Levels

Connector configuration in Hyperswitch occurs at several levels:

1. **Base Configuration**: Defined in configuration files (TOML)
2. **Environment Variables**: Can override base configuration
3. **Merchant-specific Configuration**: Stored in the database
4. **Request-level Configuration**: Provided in specific API requests

## Base Configuration

Base configuration is defined in TOML files located in the `config/` directory:

- `config/config.example.toml`: Example configuration file
- `config/development.toml`: Development environment configuration
- `config/docker_compose.toml`: Docker Compose environment configuration

### Connector Configuration Structure

In the configuration files, connector configuration is defined under the `[connectors]` section:

```toml
[connectors]
[connectors.stripe]
base_url = "https://api.stripe.com"
secrets_file = "stripeSecrets.toml"  # Optional, for local development

[connectors.adyen]
base_url = "https://checkout-test.adyen.com"
secrets_file = "adyenSecrets.toml"  # Optional, for local development

# Add your new connector configuration
[connectors.your_connector]
base_url = "https://api.your-connector.com"
secrets_file = "yourConnectorSecrets.toml"  # Optional, for local development
```

### Common Configuration Parameters

| Parameter | Description | Example |
|-----------|-------------|--------|
| `base_url` | Base URL for the connector's API | `"https://api.your-connector.com"` |
| `secondary_base_url` | Secondary base URL (if applicable) | `"https://api-secondary.your-connector.com"` |
| `api_key` | API key for authentication (not recommended in files) | Prefer environment variables |
| `secrets_file` | Path to file containing secrets (for development) | `"yourConnectorSecrets.toml"` |
| `timeout` | Request timeout in seconds | `60` |
| `webhook_source` | Source identifier for webhooks | `"your-connector"` |

### Secrets File (Development Only)

For local development, sensitive information can be stored in a secrets file. This approach should **never** be used in production:

```toml
# yourConnectorSecrets.toml
api_key = "your_api_key_here"
api_secret = "your_api_secret_here"
```

These files should be added to `.gitignore` to prevent committing secrets to the repository.

## Environment Variables

In production environments, sensitive configuration such as API keys should be provided through environment variables. Hyperswitch uses a structured naming convention for connector environment variables:

```
API_KEY_<CONNECTOR_NAME>=your_api_key_here
API_SECRET_<CONNECTOR_NAME>=your_api_secret_here
```

### Common Environment Variables

| Environment Variable | Description | Example |
|----------------------|-------------|--------|
| `API_KEY_<CONNECTOR>` | API key for authentication | `API_KEY_STRIPE=sk_test_12345` |
| `API_SECRET_<CONNECTOR>` | API secret (if applicable) | `API_SECRET_ADYEN=test_secret_key` |
| `WEBHOOK_SECRET_<CONNECTOR>` | Secret for webhook signature verification | `WEBHOOK_SECRET_STRIPE=whsec_12345` |
| `SECONDARY_API_KEY_<CONNECTOR>` | Secondary API key (if applicable) | `SECONDARY_API_KEY_ADYEN=test_key_2` |
| `BASE_URL_<CONNECTOR>` | Override base URL from config file | `BASE_URL_STRIPE=https://api.stripe.com` |

### Example Environment Variables

```bash
# Stripe configuration
API_KEY_STRIPE=sk_test_12345
WEBHOOK_SECRET_STRIPE=whsec_12345

# Adyen configuration
API_KEY_ADYEN=test_api_key
API_SECRET_ADYEN=test_secret_key
CLIENT_KEY_ADYEN=test_client_key

# Your connector configuration
API_KEY_YOUR_CONNECTOR=your_test_api_key
WEBHOOK_SECRET_YOUR_CONNECTOR=your_webhook_secret
```

## Merchant-specific Configuration

Merchant-specific connector configuration is stored in the database. This allows different merchants to use different configurations for the same connector.

### Merchant Connector Account

Merchant connector accounts are defined in the `merchant_connector_account` table with the following fields:

| Field | Description | Example |
|-------|-------------|--------|
| `merchant_id` | ID of the merchant | `"merchant_123"` |
| `connector_name` | Name of the connector | `"stripe"` |
| `connector_account_details` | JSON with account details | See below |
| `disabled` | Whether the connector is disabled | `false` |
| `test_mode` | Whether in test mode | `true` |

### Connector Account Details

The `connector_account_details` field contains a JSON object with connector-specific configuration:

```json
{
  "auth_type": "HeaderKey",
  "api_key": "merchant_specific_api_key",
  "webhook_details": {
    "webhook_secret": "merchant_specific_webhook_secret",
    "webhook_url": "https://merchant.com/webhooks/connector"
  },
  "connector_webhook_details": {
    "webhook_version": "v2"
  },
  "metadata": {
    "custom_field": "custom_value"
  }
}
```

### Creating Merchant Connector Accounts

Merchant connector accounts can be created through the Hyperswitch API:

```bash
curl -X POST \
  https://sandbox.hyperswitch.io/api/v1/account/connectors \
  -H 'Content-Type: application/json' \
  -H 'Authorization: Bearer <merchant_api_key>' \
  -d '{
    "connector_name": "your_connector",
    "connector_account_details": {
      "auth_type": "HeaderKey",
      "api_key": "merchant_api_key"
    },
    "test_mode": true
  }'
```

## Request-level Configuration

Certain configuration parameters can be provided at the request level for specific operations. This allows for greater flexibility in handling payment operations.

### Payment Request Configuration

In payment requests, connector-specific configuration can be provided in the `connector_specific_params` field:

```json
{
  "amount": 1000,
  "currency": "USD",
  "payment_method": {...},
  "connector": "your_connector",
  "connector_specific_params": {
    "your_connector": {
      "custom_param": "custom_value",
      "preferred_method": "method_a"
    }
  }
}
```

### Supporting Request-level Configuration

To support request-level configuration in your connector implementation, you'll need to handle the `connector_specific_params` in your request transformers:

```rust
impl TryFrom<&PaymentsAuthorizeRouterData> for YourConnectorPaymentsRequest {
    type Error = error_stack::Report<errors::ConnectorError>;
    
    fn try_from(item: &PaymentsAuthorizeRouterData) -> Result<Self, Self::Error> {
        // Extract connector-specific parameters if present
        let connector_specific_params = item.request.connector_specific_params
            .as_ref()
            .and_then(|params| params.your_connector.as_ref());
        
        // Use parameters in request construction
        let custom_param = connector_specific_params
            .and_then(|params| params.custom_param.clone())
            .unwrap_or_default();
        
        // Rest of the transformation
        // ...
        
        Ok(Self {
            // Use custom_param in the request
            // ...
        })
    }
}
```

## Configuration Best Practices

### 1. Security

- **Never** store API keys or secrets in configuration files for production environments
- Use environment variables for sensitive information
- Encrypt sensitive data in the database
- Rotate API keys regularly
- Use different API keys for test and production environments

### 2. Testing

- Create separate configurations for test and production environments
- Test configuration changes in sandbox environments before deploying to production
- Implement validation for configuration parameters
- Handle configuration errors gracefully

### 3. Documentation

- Document all configuration parameters with examples
- Provide clear instructions for configuring the connector
- Document environment variable requirements
- Include example configurations for different scenarios

## Example Connector Configuration

### Base Configuration

```toml
# In config/development.toml
[connectors.your_connector]
base_url = "https://api.your-connector.com/sandbox"
timeout = 60
```

### Environment Variables

```bash
# Production environment
API_KEY_YOUR_CONNECTOR=live_api_key_xyz
WEBHOOK_SECRET_YOUR_CONNECTOR=live_webhook_secret

# Development environment
DEV_API_KEY_YOUR_CONNECTOR=test_api_key_xyz
DEV_WEBHOOK_SECRET_YOUR_CONNECTOR=test_webhook_secret
```

### Merchant Configuration

```json
{
  "connector_name": "your_connector",
  "connector_account_details": {
    "auth_type": "HeaderKey",
    "api_key": "merchant_specific_api_key",
    "webhook_details": {
      "webhook_secret": "merchant_specific_webhook_secret"
    },
    "metadata": {
      "merchant_id": "merchant_identifier_in_your_connector",
      "terminal_id": "pos_terminal_identifier"
    }
  },
  "test_mode": false
}
```

## Connector-specific Configuration

Different connectors may have unique configuration requirements. Here are examples for common connector types:

### Card Payment Processors

```json
{
  "auth_type": "HeaderKey",
  "api_key": "card_processor_api_key",
  "webhook_details": {
    "webhook_secret": "webhook_signing_secret"
  },
  "metadata": {
    "terminal_id": "pos_terminal_id",
    "merchant_category_code": "1234"
  }
}
```

### Bank Payment Processors

```json
{
  "auth_type": "OAuth",
  "client_id": "oauth_client_id",
  "client_secret": "oauth_client_secret",
  "webhook_details": {
    "webhook_secret": "webhook_signing_secret"
  },
  "connector_webhook_details": {
    "return_url": "https://merchant.com/return"
  }
}
```

### Digital Wallet Providers

```json
{
  "auth_type": "HeaderKey",
  "api_key": "wallet_provider_api_key",
  "metadata": {
    "merchant_id": "wallet_merchant_id",
    "supported_currencies": "USD,EUR,GBP"
  }
}
```

## Configuration Validation

Implement validation for your connector configuration to ensure it is complete and correct:

```rust
impl ConnectorValidation for YourConnector {
    fn validate_connector_config(&self, connector_config: &ConnectorConfig) -> CustomResult<(), errors::ConnectorError> {
        // Validate required configuration parameters
        if connector_config.api_key.is_empty() {
            return Err(errors::ConnectorError::MissingRequiredField {
                field_name: "api_key".to_string(),
            }.into());
        }
        
        // Validate other configuration parameters
        // ...
        
        Ok(())
    }
}
```

## Troubleshooting Configuration Issues

### Common Issues

1. **Missing API Keys**: Ensure API keys are correctly set in environment variables or merchant configuration
2. **Incorrect Base URL**: Verify the base URL is correct for the environment (sandbox vs. production)
3. **Authentication Failures**: Check that the authentication parameters are correct
4. **Webhook Configuration**: Ensure webhook secrets and URLs are correctly configured

### Debugging Tips

1. **Enable Debug Logging**: Set `RUST_LOG=debug` to see detailed logs
2. **Check Environment Variables**: Verify that environment variables are correctly set
3. **Validate Configuration**: Use connector validation to check configuration
4. **Test API Credentials**: Test API credentials with the connector's API directly

## Conclusion

Proper configuration is essential for connectors to function correctly. By following the guidelines in this document, you can ensure your connector is correctly configured and ready for use in Hyperswitch.

## Next Steps

1. Review the [Connector Interface Requirements](./connector_interface_guide.md) to understand the interface requirements
2. Follow the [Connector Implementation Guide](./connector_implementation_guide.md) to implement your connector
3. Use the [Connector Testing Guide](./connector_testing_guide.md) to test your connector

## See Also

- [Hyperswitch Connectors Overview](./overview.md)
- [Error Handling Guidelines](./error_handling.md)