# Common Enums Crate Overview

## Purpose

The `common_enums` crate provides a centralized collection of enumerations shared across the Hyperswitch platform. These enumerations serve as standardized data types for various aspects of payment processing, including payment statuses, currencies, connector types, and geographical information. The crate is designed to eliminate redundancy and ensure consistency in how these common enumerations are defined and used throughout the system.

## Key Modules

### enums.rs

This core module defines the primary enumeration types used throughout the Hyperswitch platform:

- **Status Enumerations**:
  - `AttemptStatus`: Represents the current status of a payment attempt (e.g., `Started`, `Authorized`, `Charged`, `Failed`)
  - `IntentStatus`: Represents the status of a payment intent (e.g., `Succeeded`, `Failed`, `Processing`)
  - `CaptureStatus`: Represents the status of a capture operation
  - `RefundStatus`: Represents the status of a refund
  - `DisputeStatus`: Represents the status of a dispute
  - `FraudCheckStatus`: Represents the status of a fraud check

- **Payment Method Enumerations**:
  - `PaymentMethod`: High-level payment method categories (e.g., `Card`, `Wallet`, `BankRedirect`)
  - `PaymentMethodType`: Specific payment method types (e.g., `CreditCard`, `GooglePay`, `ApplePay`)
  - `CardDiscovery`: Methods by which a card is discovered during payment

- **Geographical Enumerations**:
  - `Currency`: ISO 4217 currency codes with validation and conversion utilities
  - `CountryAlpha2`: ISO 3166-1 alpha-2 country codes
  - `CountryAlpha3`: ISO 3166-1 alpha-3 country codes
  - `Country`: Full country names

- **Authentication Enumerations**:
  - `AuthenticationType`: Types of authentication (e.g., `ThreeDs`, `NoThreeDs`)
  - `AuthenticationStatus`: Status of authentication processes
  - `TransactionStatus`: Authentication transaction statuses

- **Configuration Enumerations**:
  - `CaptureMethod`: Methods for capturing payments (e.g., `Automatic`, `Manual`)
  - `ConnectorType`: Types of connectors (e.g., `PaymentProcessor`, `PaymentVas`)
  - `FutureUsage`: Options for storing payment methods for future use

### connector_enums.rs

This module defines enumerations specific to payment connectors and their routing:

- **Connector Enumerations**:
  - `Connector`: Comprehensive list of all supported payment connectors
  - `RoutableConnectors`: Subset of connectors eligible for payment routing

The module also provides utility methods to determine connector capabilities, such as:
- Support for instant payouts
- Support for access tokens
- Support for file storage
- Support for separate authentication
- Requirements for dispute defense
- Extended authorization support for specific payment methods

### transformers.rs

This module provides transformation functions for enumeration types:

- Country code conversions between different formats:
  - Alpha-2 to Alpha-3
  - Alpha-2 to full country name
  - Alpha-3 to full country name
  - Numeric to full country name

- Custom serialization and deserialization for geographic codes:
  - `alpha2_country_code`: Serialization/deserialization using ISO 3166-1 alpha-2 format
  - `alpha3_country_code`: Serialization/deserialization using ISO 3166-1 alpha-3 format
  - `numeric_country_code`: Serialization/deserialization using numeric country codes

- Status transformations:
  - Conversion between `AttemptStatus` and `IntentStatus`
  - Boolean conversions for request types

## Configuration Options

The crate supports several feature flags that modify its functionality:

- `dummy_connector`: Enables dummy connectors for testing
- `openapi`: Enables OpenAPI schema generation
- `payouts`: Enables payout-specific enumerations
- `v2`: Enables v2 API-specific enumerations

## Key Features

1. **Standardized Enumeration Types**: Provides a single source of truth for enumeration types used across the platform
2. **Rich Geographic Data**: Complete ISO-standard country and currency information with conversion utilities
3. **Connector Capability Introspection**: Methods to determine connector capabilities based on payment method
4. **Serialization Support**: Customized serialization for compatibility with external systems
5. **Status Tracking**: Comprehensive set of enumerations for tracking payment lifecycle states

## Usage Examples

### Using Currency Conversion Functions

```rust
use common_enums::Currency;

fn calculate_display_amount(amount: i64, currency: Currency) -> String {
    // Convert the amount to its base denomination based on currency
    currency.to_currency_base_unit(amount).unwrap_or_default()
}

// Example: 1000 in USD becomes "10.00"
let display_amount = calculate_display_amount(1000, Currency::USD);

// Example: 1000 in JPY (zero-decimal currency) becomes "1000"
let display_amount_jpy = calculate_display_amount(1000, Currency::JPY);
```

### Determining Connector Capabilities

```rust
use common_enums::{Connector, PaymentMethod};

fn supports_token_based_payments(connector: Connector, payment_method: PaymentMethod) -> bool {
    connector.supports_access_token(payment_method)
}

// Check if Stripe supports token-based card payments
let supports_tokens = supports_token_based_payments(Connector::Stripe, PaymentMethod::Card);
```

### Country Code Conversions

```rust
use common_enums::{Country, CountryAlpha2};

// Convert from Alpha-2 to full country
let country = Country::from_alpha2(CountryAlpha2::US);

// Convert from country to numeric code
let numeric_code = country.to_numeric(); // Returns 840 for US
```

## Integration with Other Crates

The `common_enums` crate is a foundational dependency for many other crates in the Hyperswitch ecosystem:

- **api_models**: Uses the enumerations for API request/response definitions
- **router**: Uses status enumerations for payment flow control
- **hyperswitch_domain_models**: Uses the enumerations for domain model definitions
- **hyperswitch_connectors**: Uses connector enumerations to implement connector integrations
- **storage_impl**: Uses the enumerations for database operations

## Testing Approach

The crate includes comprehensive unit tests, particularly for:

- Country code conversions
- Currency formatting and decimal handling
- Serialization/deserialization of enumeration types

## Maintenance Considerations

When updating this crate, consider:

1. **Backward Compatibility**: Changes to enumerations may affect serialization formats and database values
2. **Cross-Crate Impact**: Updates may require coordinated changes in dependent crates
3. **Documentation**: Ensure all new enumerations include proper documentation for OpenAPI schema generation
4. **Testing**: Add tests for any new conversions or transformations

## Related Documentation

- [Router Crate Overview](../router/overview.md)
- [API Models Overview](../api_models/overview.md)
- [Hyperswitch Domain Models Overview](../hyperswitch_domain_models/overview.md)
