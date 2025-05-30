# Common Types Crate Overview

## Purpose

The `common_types` crate serves as a shared type system for the Hyperswitch payment orchestration platform. Its primary responsibilities include:

1. Providing standardized data types that are shared between API request/response models and database models
2. Defining primitives and domain-specific types for payment processing
3. Implementing serialization/deserialization and database mapping for these shared types
4. Supporting API documentation generation through Utoipa annotations

By centralizing these shared types, the crate helps maintain consistency across the platform's layers, reduces code duplication, and ensures proper type handling across API boundaries and persistent storage.

## Key Modules

### payments.rs

This core module defines types related to payment processing:

- **Split Payment Types**: Handles split payments for various connectors (Stripe, Adyen, Xendit)
  - `SplitPaymentsRequest`: Enumeration for different connector-specific split payment requests
  - `StripeSplitPaymentRequest`: Struct for Stripe's split payment information
  - `XenditSplitRequest`: Variants for Xendit's single and multiple splits
  - `ConnectorChargeResponseData`: Response data for connector-specific charge operations

- **Decision Management Types**:
  - `DecisionManagerRecord`: Configuration for decision-based routing
  - `ConditionalConfigs`: Conditional configurations for payment routing decisions
  - `AuthenticationConnectorAccountMap`: Maps authentication products to connector accounts

### domain.rs

This module provides core domain entities that represent fundamental business concepts:

- **Split Payment Domain Types**:
  - `AdyenSplitData`: Split payment information for Adyen
  - `AdyenSplitItem`: Individual split item for Adyen payments
  - `XenditSplitSubMerchantData`: Sub-merchant data for Xendit split payments

These types form the basis of Hyperswitch's domain model for split payment processing.

### primitive_wrappers.rs

This module defines wrapper types around primitive data types to provide:

- Type safety through specialized wrapper types
- Custom serialization/deserialization behavior
- SQL integration for database operations
- Schema documentation for API generation

### customers.rs

Contains customer-related types that support customer management functionality:

- Customer data structures
- Customer payment method associations
- Customer metadata and attributes

### payment_methods.rs

Defines data structures related to various payment methods:

- Payment method details
- Card information types
- Wallet-specific data structures
- Bank transfer details
- Alternative payment method types

### refunds.rs

Provides types for refund processing:

- Refund request structures
- Refund response data
- Refund status tracking

### consts.rs

Contains constant values and data used throughout the system:

- Default field values
- Rate limits
- Timeouts
- Validation constraints

## Key Features

1. **Diesel Integration**: Types implement Diesel's `FromSqlRow` and `AsExpression` traits for seamless database interaction, allowing these structures to be directly used with Diesel's ORM.

2. **Serde Support**: All types implement Serde's `Serialize` and `Deserialize` traits, enabling JSON serialization for API communication and configuration storage.

3. **API Documentation**: Types are annotated with Utoipa's `ToSchema` trait, facilitating automatic OpenAPI documentation generation.

4. **Type Safety**: Wrapper types around primitives provide compile-time type safety and domain-specific validation.

5. **Split Payment Support**: Comprehensive support for various split payment models across different payment processors.

6. **Decision Engine Integration**: Integration with the `euclid` crate for rule-based decision making in payment routing.

## Data Patterns

### SQL Type Integration

The crate extensively uses a pattern of deriving both Diesel traits and Serde traits to allow seamless transition between database and API layers:

```rust
#[derive(
    Serialize, Deserialize, Debug, Clone, PartialEq, Eq, FromSqlRow, AsExpression, ToSchema,
)]
#[diesel(sql_type = Jsonb)]
pub struct TypeName {
    // Fields...
}
impl_to_sql_from_sql_json!(TypeName);
```

This pattern enables:
1. JSON serialization/deserialization via Serde
2. Database storage and retrieval via Diesel
3. API documentation generation via Utoipa

### Enumeration Pattern

Complex data models that vary by connector are often represented as enums with connector-specific variants:

```rust
#[derive(/* traits */)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorSpecificType {
    StripeVariant(StripeSpecificStruct),
    AdyenVariant(AdyenSpecificStruct),
    XenditVariant(XenditSpecificStruct),
}
```

This approach allows for type-safe handling of connector-specific behaviors while maintaining a unified interface.

## Usage Examples

### Working with Split Payments

```rust
use common_types::payments::{SplitPaymentsRequest, StripeSplitPaymentRequest};
use common_enums::enums::PaymentChargeType;
use common_utils::types::MinorUnit;

// Create a Stripe split payment request
let stripe_split = StripeSplitPaymentRequest {
    charge_type: PaymentChargeType::Direct,
    application_fees: MinorUnit::new(500), // 5.00 in the currency's base unit
    transfer_account_id: "acct_123456".to_string(),
};

// Use it in a general split payment request
let split_request = SplitPaymentsRequest::StripeSplitPayment(stripe_split);

// This can now be included in a payment request or stored in the database
```

### Using Decision Manager Types

```rust
use common_types::payments::{ConditionalConfigs, DecisionManagerRecord};
use euclid::frontend::ast::Program;

// Create conditional configuration for payment routing
let conditional_config = ConditionalConfigs {
    override_3ds: Some(common_enums::AuthenticationType::ThreeDs),
};

// Create a decision manager with a routing program
let decision_manager = DecisionManagerRecord {
    name: "Priority Routing".to_string(),
    program: Program::new(/* routing rules */),
    created_at: chrono::Utc::now().timestamp(),
};

// This decision manager can now be used to route payments based on defined rules
```

## Integration with Other Crates

The `common_types` crate is a foundational dependency for many other crates in the Hyperswitch ecosystem:

- **api_models**: Uses common types as the basis for API request/response definitions
- **router**: Uses shared types for internal data structures and database interactions
- **hyperswitch_domain_models**: Builds upon these types to create higher-level domain entities
- **hyperswitch_connectors**: Uses these types for connector request/response mapping
- **storage_impl**: Uses these types for database operations and schema definitions

The crate depends on several other Hyperswitch crates:

- **common_enums**: For enumeration types (currencies, countries, payment statuses, etc.)
- **common_utils**: For utility functions and helper types
- **euclid**: For decision engine integration

## Configuration Options

The crate supports several feature flags that modify its functionality:

- **v1**: Enables compatibility with v1 API structures
- **v2**: Enables compatibility with v2 API structures

These features correspond to similar features in dependencies, particularly `common_utils`.

## Testing Approach

The types in this crate are primarily tested through:

1. Unit tests for serialization/deserialization
2. Database integration tests for SQL mapping
3. Integration tests in dependent crates that validate correct interaction

## Maintenance Considerations

When updating this crate, consider:

1. **Database Compatibility**: Changes to types used in database operations may require migrations
2. **API Compatibility**: Changes may affect API request/response structures
3. **Cross-Crate Impact**: Updates may require coordinated changes in dependent crates
4. **Documentation**: Ensure new types include proper documentation annotations for OpenAPI

## Related Documentation

- [Router Crate Overview](../router/overview.md)
- [Common Enums Overview](../common_enums/overview.md)
- [API Models Overview](../api_models/overview.md)
- [Hyperswitch Domain Models Overview](../hyperswitch_domain_models/overview.md)
