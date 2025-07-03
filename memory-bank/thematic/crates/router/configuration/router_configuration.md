---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

# Router Configuration Options

---
**Parent:** [Router Overview](../overview.md)  
**Related Files:**
- [Feature Flags](./feature_flags.md)
- [Routing Strategies](./routing_strategies.md)
- [Payment Flows](../flows/payment_flows.md)
- [Refund Flows](../flows/refund_flows.md)
- [Webhook Flows](../flows/webhook_flows.md)
---

[← Back to Router Overview](../overview.md)

## Overview

The router component of Hyperswitch is highly configurable, allowing for customization of its behavior to meet specific requirements. This document details the various configuration options available for the router, covering general settings, performance tuning, security configurations, and integration options.

## Configuration Sources

Router configuration comes from several sources:

1. **Configuration Files**: TOML files in the `config/` directory
2. **Environment Variables**: Override file-based configurations
3. **Database Settings**: Merchant-specific configurations stored in the database
4. **Runtime Configurations**: Configurations that can be changed during runtime via admin APIs

## General Configuration

### Core Settings

```toml
[server]
port = 8080                 # HTTP port for the server
host = "127.0.0.1"         # Host address to bind to
workers = 4                # Number of worker threads
request_body_limit = "10MB" # Maximum request body size
```

- **port**: The HTTP port on which the server listens
- **host**: The host address to bind to
- **workers**: Number of worker threads for processing requests
- **request_body_limit**: Maximum allowed size for HTTP request bodies

### Logging Configuration

```toml
[log]
level = "info"             # Logging level (debug, info, warn, error)
file = "/var/log/hyperswitch/router.log" # Log file path
console = true             # Whether to log to console
json_format = true         # Whether to use JSON format for logs
```

- **level**: The logging level (debug, info, warn, error)
- **file**: Path to the log file
- **console**: Whether to output logs to the console
- **json_format**: Whether to format logs as JSON (useful for log aggregation)

## Performance Configuration

### Connection Pooling

```toml
[database]
pool_size = 10             # Database connection pool size
connection_timeout = "5s"  # Database connection timeout
idle_timeout = "10m"       # Idle connection timeout
```

- **pool_size**: Number of database connections to maintain in the pool
- **connection_timeout**: Timeout for establishing database connections
- **idle_timeout**: How long idle connections are kept in the pool

### Caching

```toml
[cache]
enabled = true             # Whether caching is enabled
ttl = "5m"                 # Default time-to-live for cached items
redis_url = "redis://localhost:6379/0" # Redis connection URL
```

- **enabled**: Whether caching is enabled
- **ttl**: Default time-to-live for cached items
- **redis_url**: Redis connection URL for distributed caching

### Timeouts

```toml
[timeouts]
http_request = "30s"       # HTTP client request timeout
connector_request = "60s"  # Payment connector request timeout
webhook_delivery = "15s"   # Webhook delivery timeout
```

- **http_request**: Timeout for general HTTP client requests
- **connector_request**: Timeout specifically for payment connector requests
- **webhook_delivery**: Timeout for delivering webhooks to merchants

## Security Configuration

### API Authentication

```toml
[api_auth]
api_key_header = "x-api-key" # Header name for API key
jwt_secret = "your-secret-key" # Secret for JWT authentication
token_expiry = "24h"      # JWT token expiry time
```

- **api_key_header**: HTTP header name for API key authentication
- **jwt_secret**: Secret key for JWT token signing/validation
- **token_expiry**: Expiry time for JWT tokens

### Encryption

```toml
[encryption]
master_key = "your-master-encryption-key" # Master encryption key
key_rotation_interval = "90d" # Key rotation interval
```

- **master_key**: Master key for encrypting sensitive data
- **key_rotation_interval**: Interval for encryption key rotation

### Rate Limiting

```toml
[rate_limit]
enabled = true             # Whether rate limiting is enabled
request_limit = 100        # Requests per interval
interval = "1m"            # Rate limit interval
blocked_ips = ["1.2.3.4"] # IPs to block entirely
```

- **enabled**: Whether rate limiting is enabled
- **request_limit**: Maximum number of requests in the interval
- **interval**: Time interval for rate limiting
- **blocked_ips**: List of IP addresses to block entirely

## Connector Configuration

### General Connector Settings

```toml
[connectors]
default_connector = "stripe" # Default connector if none specified
connector_timeout = "30s"   # Connector request timeout
retry_attempts = 3         # Number of retry attempts
retry_interval = "1s"      # Interval between retries
```

- **default_connector**: Default payment connector to use if none is specified
- **connector_timeout**: Timeout for connector requests
- **retry_attempts**: Number of retry attempts for failed connector requests
- **retry_interval**: Interval between retry attempts

### Connector-Specific Configuration

```toml
[connectors.stripe]
api_key = "${STRIPE_API_KEY}" # Stripe API key
webhook_secret = "${STRIPE_WEBHOOK_SECRET}" # Stripe webhook secret
base_url = "https://api.stripe.com" # Stripe API base URL

[connectors.adyen]
api_key = "${ADYEN_API_KEY}" # Adyen API key
merchant_account = "${ADYEN_MERCHANT_ACCOUNT}" # Adyen merchant account
base_url = "https://checkout-test.adyen.com" # Adyen API base URL
```

- Each connector has its own section with connector-specific settings
- Environment variable interpolation is supported (e.g., `${STRIPE_API_KEY}`)

## Webhook Configuration

### Incoming Webhooks

```toml
[webhooks.incoming]
ip_filtering = true        # Whether to filter incoming webhooks by IP
allowed_ips = ["1.2.3.0/24"] # Allowed IP ranges for webhooks
signature_verification = true # Whether to verify webhook signatures
```

- **ip_filtering**: Whether to filter incoming webhooks by source IP
- **allowed_ips**: List of allowed IP ranges for incoming webhooks
- **signature_verification**: Whether to verify webhook signatures

### Outgoing Webhooks

```toml
[webhooks.outgoing]
max_retries = 5            # Maximum number of retry attempts
retry_interval = "5m"      # Initial interval between retries
backoff_factor = 2.0       # Exponential backoff factor for retries
retry_timeout = "48h"      # Maximum time to keep retrying
```

- **max_retries**: Maximum number of retry attempts for failed webhook deliveries
- **retry_interval**: Initial interval between retry attempts
- **backoff_factor**: Factor by which the retry interval increases with each attempt
- **retry_timeout**: Maximum time to keep retrying webhook delivery

## Merchant-Specific Configuration

Merchant-specific configurations are stored in the database and can override global settings. These include:

- **Webhook endpoints**: Merchant-specific webhook delivery endpoints
- **Retry policies**: Custom webhook retry policies for each merchant
- **Rate limits**: Merchant-specific API rate limits
- **Connector preferences**: Preferred payment connectors for the merchant
- **Feature flags**: Merchant-specific feature flags

## Environment Variables

All configuration settings can be overridden using environment variables with a specific naming convention:

- Convert the TOML path to uppercase and replace dots with underscores
- For example, to override `[server].port`, use `SERVER_PORT=8081`

Example environment variables:

```sh
# Server configuration
SERVER_PORT=8081
SERVER_HOST=0.0.0.0

# Database configuration
DATABASE_URL=postgres://user:pass@localhost/hyperswitch
DATABASE_POOL_SIZE=20

# Connector API keys
CONNECTORS_STRIPE_API_KEY=sk_test_xyz
CONNECTORS_ADYEN_API_KEY=AQEm...wd#
```

## Dynamic Configuration

Some configuration options can be changed dynamically at runtime through the admin API:

- **Feature flags**: Enable/disable features
- **Rate limits**: Adjust rate limiting settings
- **Connector settings**: Update connector credentials or settings
- **Logging levels**: Change logging verbosity

These changes take effect immediately without requiring a service restart.

## Configuration File Example

Here's a complete example of a router configuration file:

```toml
[server]
port = 8080
host = "0.0.0.0"
workers = 8
request_body_limit = "10MB"

[log]
level = "info"
file = "/var/log/hyperswitch/router.log"
console = true
json_format = true

[database]
url = "postgres://hyperswitch:password@localhost/hyperswitch"
pool_size = 20
connection_timeout = "5s"
idle_timeout = "10m"

[cache]
enabled = true
ttl = "5m"
redis_url = "redis://localhost:6379/0"

[timeouts]
http_request = "30s"
connector_request = "60s"
webhook_delivery = "15s"

[api_auth]
api_key_header = "x-api-key"
jwt_secret = "${JWT_SECRET}"
token_expiry = "24h"

[encryption]
master_key = "${ENCRYPTION_MASTER_KEY}"
key_rotation_interval = "90d"

[rate_limit]
enabled = true
request_limit = 100
interval = "1m"

[connectors]
default_connector = "stripe"
connector_timeout = "30s"
retry_attempts = 3
retry_interval = "1s"

[connectors.stripe]
api_key = "${STRIPE_API_KEY}"
webhook_secret = "${STRIPE_WEBHOOK_SECRET}"
base_url = "https://api.stripe.com"

[webhooks.incoming]
ip_filtering = true
allowed_ips = ["1.2.3.0/24"]
signature_verification = true

[webhooks.outgoing]
max_retries = 5
retry_interval = "5m"
backoff_factor = 2.0
retry_timeout = "48h"
```

## Configuration Best Practices

1. **Use environment variables for secrets**: Never store API keys, passwords, or other secrets directly in configuration files
2. **Version control configuration templates**: Store configuration templates in version control, but not actual configurations with sensitive values
3. **Use different configurations for environments**: Maintain separate configurations for development, testing, and production
4. **Document all configuration changes**: Keep a log of configuration changes, especially in production
5. **Validate configurations**: Use the router's configuration validation tool to check configuration files before deployment
6. **Monitor configuration impact**: After changing configurations, monitor system performance to ensure the changes have the desired effect

## See Also

- [Feature Flags Documentation](./feature_flags.md)
- [Routing Strategies Documentation](./routing_strategies.md)
- [Payment Flows Documentation](../flows/payment_flows.md)
- [Refund Flows Documentation](../flows/refund_flows.md)
- [Webhook Flows Documentation](../flows/webhook_flows.md)

## Document History
| Date | Changes |
|------|----------|
| 2025-05-27 | Initial version |