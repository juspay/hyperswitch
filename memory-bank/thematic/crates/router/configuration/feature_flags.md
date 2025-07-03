# Router Feature Flags

---
**Parent:** [Router Overview](../overview.md)  
**Last Updated:** 2025-05-20  
**Related Files:**
- [Routing Strategies](./routing_strategies.md)
- [Dependencies](../architecture/dependencies.md)
---

[← Back to Router Overview](../overview.md)

## Overview

The router crate makes extensive use of Cargo feature flags to enable conditional compilation of different functionalities. This approach allows for customization of the Hyperswitch platform, enabling or disabling specific capabilities based on deployment needs. This document details the available feature flags and their effects on the system.

## Feature Flag Categories

The feature flags in the router crate can be categorized into several groups:

### API Versions

These flags control which API versions are enabled:

- **`v1`**: Enables the v1 API endpoints
  - Default: Enabled
  - Impact: Includes all v1 API routes and handlers
  - Dependencies: Core v1 request/response models

- **`v2`**: Enables the v2 API endpoints
  - Default: Enabled in newer releases
  - Impact: Includes all v2 API routes and handlers
  - Dependencies: Core v2 request/response models
  - Incompatibilities: None, can be enabled alongside v1

### Payment Processors (Connectors)

These flags control which payment processors are compiled into the binary:

- **`stripe`**: Enables Stripe payment processor integration
  - Default: Enabled
  - Impact: Includes Stripe connector implementation
  - Dependencies: Stripe-specific models and transformers

- **`adyen`**: Enables Adyen payment processor integration
  - Default: Enabled
  - Impact: Includes Adyen connector implementation
  - Dependencies: Adyen-specific models and transformers

- **`checkout`**: Enables Checkout.com payment processor integration
  - Default: Enabled
  - Impact: Includes Checkout.com connector implementation
  - Dependencies: Checkout-specific models and transformers

- **`worldpay`**: Enables Worldpay payment processor integration
  - Default: Enabled
  - Impact: Includes Worldpay connector implementation
  - Dependencies: Worldpay-specific models and transformers

- Additional connectors like **`paypal`**, **`authorize_net`**, **`cybersource`**, etc.

### Processing Modes

These flags configure the processing focus of the application:

- **`oltp`**: Optimizes for Online Transaction Processing
  - Default: Enabled
  - Impact: Configures the application for real-time transaction processing
  - Dependencies: None specific

- **`olap`**: Optimizes for Online Analytical Processing
  - Default: Disabled
  - Impact: Enables additional analytics capabilities
  - Dependencies: May require additional database configurations

### Optional Features

These flags enable or disable major functional areas:

- **`frm`**: Fraud Risk Management capabilities
  - Default: Disabled
  - Impact: Enables fraud detection and prevention systems
  - Dependencies: FRM-specific models and external integrations

- **`payouts`**: Payout functionality (transfers to recipients)
  - Default: Disabled
  - Impact: Enables payout APIs and processing
  - Dependencies: Payout-specific models and connector implementations

- **`recon`**: Reconciliation functionality
  - Default: Disabled
  - Impact: Enables reconciliation systems for matching transactions
  - Dependencies: Reconciliation models and workflows

- **`dynamic_routing`**: Advanced routing capabilities
  - Default: Disabled
  - Impact: Enables sophisticated payment routing strategies
  - Dependencies: euclid crate and rule engine integrations

- **`email`**: Email notification functionality
  - Default: Disabled
  - Impact: Enables email sending capabilities
  - Dependencies: Email client libraries and templates

### Storage Options

These flags control storage backend options:

- **`kv_store`**: Enables key-value store support
  - Default: Disabled
  - Impact: Allows using key-value stores for certain data
  - Dependencies: KV store client libraries

- **`redis_cluster`**: Enables Redis Cluster support
  - Default: Disabled
  - Impact: Configures Redis for cluster mode operation
  - Dependencies: Redis cluster client features

## Default Configuration

The router crate comes with a default set of feature flags enabled:

```toml
[features]
default = ["v1", "v2", "stripe", "adyen", "checkout", "worldpay", "oltp"]
```

This default configuration provides:
- Both v1 and v2 API versions
- Support for major payment processors
- OLTP-optimized configuration

## Custom Configurations

Custom configurations can be created for specific deployment scenarios:

### Minimal Configuration

```toml
[features]
minimal = ["v2", "stripe", "adyen", "oltp"]
```

This minimal configuration includes only essential features:
- Only v2 API
- Limited connector support
- Basic transaction processing

### Analytics Configuration

```toml
[features]
analytics = ["v1", "v2", "stripe", "adyen", "checkout", "worldpay", "oltp", "olap", "recon"]
```

This analytics-focused configuration includes:
- Both API versions
- Major payment processors
- Both OLTP and OLAP capabilities
- Reconciliation functionality

### Full-Featured Configuration

```toml
[features]
full = ["v1", "v2", "stripe", "adyen", "checkout", "worldpay", "paypal", "authorize_net", 
        "cybersource", "oltp", "frm", "payouts", "recon", "dynamic_routing", "email"]
```

This configuration enables all available features:
- All API versions
- Comprehensive connector support
- All optional features

## Compile-Time vs. Runtime Features

It's important to distinguish between compile-time feature flags and runtime feature toggles:

- **Compile-Time Features** (Cargo features):
  - Set at build time
  - Cannot be changed without rebuilding
  - Control code inclusion in the binary
  - Affect binary size and capabilities

- **Runtime Features** (Feature toggles):
  - Set in configuration files or database
  - Can be changed without rebuilding
  - Control behavior of already-compiled code
  - Used for operational toggles and A/B testing

The router crate uses both approaches:
- Cargo features for significant functionality modules
- Runtime toggles for operational concerns

## Implementation Details

### Feature Guard Pattern

Feature-gated code typically follows this pattern:

```rust
#[cfg(feature = "feature_name")]
mod feature_implementation {
    // Implementation details...
}

#[cfg(not(feature = "feature_name"))]
mod feature_implementation {
    // Stub implementation or alternative...
}
```

### Conditional Routes

API routes are conditionally registered based on features:

```rust
#[cfg(feature = "v2")]
pub fn configure_v2_routes(config: &mut web::ServiceConfig) {
    // V2 route configuration...
}
```

### Feature Dependencies

Some features depend on others and are automatically enabled:

```toml
[features]
dynamic_routing = ["euclid_integration"]
euclid_integration = []
```

## Best Practices

When working with feature flags:

1. **Clear Documentation**: Each feature should be clearly documented
2. **Minimal Dependencies**: Features should have minimal dependencies on other features
3. **Clean Interfaces**: Feature boundaries should be well-defined
4. **Testing**: Each feature combination should be tested
5. **Performance Impact**: Consider the performance impact of each feature

## See Also

- [Routing Strategies Documentation](./routing_strategies.md)
- [Dependencies Documentation](../architecture/dependencies.md)
