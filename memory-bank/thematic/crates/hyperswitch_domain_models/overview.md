# Hyperswitch Domain Models Crate Overview

The `hyperswitch_domain_models` crate provides the core domain model definitions for the Hyperswitch payment orchestration platform. It serves as the bridge between the API layer (defined in `api_models`) and the database layer (defined in `diesel_models`), encapsulating business logic, validation rules, and data transformations.

## Purpose

The `hyperswitch_domain_models` crate is responsible for:

1. Defining the core domain entities and value objects used throughout the application
2. Implementing business logic and validation rules for these entities
3. Converting between API request/response models and database models
4. Managing data encryption and decryption for sensitive fields
5. Supporting versioning (v1/v2) via feature flags
6. Providing type-safe abstractions over payment processing concepts

## Key Modules

### lib.rs

The core module that exports all the domain models and provides utility traits:

- **ApiModelToDieselModelConvertor**: Trait for converting between API models and diesel models
- **RemoteStorageObject**: Generic wrapper around referenced objects
- **ForeignIDRef**: Trait for objects that can be referenced by foreign IDs

### payments.rs

Defines the core payment entities for processing payments:

- **PaymentIntent**: Central entity representing a payment intent with all its associated data
- **AmountDetails**: Comprehensive structure for amount-related data (order amount, tax, shipping, etc.)
- **HeaderPayload**: Container for HTTP header information related to payments
- **VaultOperation/VaultData**: Models for vault operations and stored payment data

### payments/payment_attempt.rs

Models for payment attempts:

- **PaymentAttempt**: Represents a single attempt to process a payment
- **AttemptAmountDetails**: Payment amount details specific to an attempt
- **ErrorDetails**: Structured error information for payment attempts

### payments/payment_intent.rs

Additional models and utilities for payment intents:

- **PaymentIntentUpdate**: Structure for updating payment intents
- **DecryptedPaymentIntent**: Representation of a payment intent with decrypted sensitive data

### customer.rs

Models related to customer data:

- **Customer**: Represents a customer in the system
- **CustomerCreate**: Model for creating new customers

### merchant_account.rs and merchant_connector_account.rs

Models for merchant accounts and their connector integrations:

- **MerchantAccount**: Represents a merchant using the platform
- **MerchantConnectorAccount**: Represents a merchant's integration with a payment connector

## Configuration Options

The crate supports several feature flags:

- **v1/v2**: Version-specific implementations (mutually exclusive)
- **encryption_service**: Enables encryption services for PII data
- **olap**: Enables analytics-related models and features
- **payouts**: Enables payout-related models and features
- **frm**: Enables fraud and risk management models
- **revenue_recovery**: Enables revenue recovery models
- **customer_v2**: Enables v2 customer models
- **payment_methods_v2**: Enables v2 payment methods
- **refunds_v2**: Enables v2 refunds

## Key Features

### Domain Model Versioning

The crate supports multiple API versions (v1 and v2) through feature flags:

- **V1 Models**: Original implementation with simpler structure
- **V2 Models**: Enhanced models with more features and better organization

### Data Encryption

Built-in support for encrypting sensitive data:

- **Encryption Annotations**: Fields marked with `#[encrypt]` are automatically encrypted
- **Type Encryption**: The `ToEncryption` derive macro handles encryption logistics
- **PII Masking**: Integration with the `masking` crate for proper PII handling

### API-to-Database Model Conversion

Comprehensive conversion between API and database layers:

- **Explicit Conversions**: Each model implements conversion logic between layers
- **Type Safety**: Conversion maintains type safety and handles validation
- **Feature-Flag Awareness**: Conversions respect feature flags for different versions

### Business Logic Encapsulation

Contains core business rules for the payment system:

- **Validation Logic**: Validates model state and transitions
- **Amount Calculations**: Handles complex amount calculations (tax, shipping, etc.)
- **Status Management**: Manages payment status transitions and validations

## Usage Examples

### Creating a Payment Intent (V2)

```rust
let payment_intent = PaymentIntent::create_domain_model_from_request(
    &payment_id,
    &merchant_context,
    &profile,
    request,
    decrypted_payment_intent,
).await?;
```

### Converting Models Between Layers

```rust
// Converting from API model to domain model
let feature_metadata = FeatureMetadata::convert_from(api_feature_metadata);

// Converting back to API model
let api_metadata = feature_metadata.convert_back();
```

### Working with Amount Calculations

```rust
// Calculate net amount for a payment
let amount_details = AmountDetails { 
    order_amount: 1000.into(),
    currency: common_enums::Currency::USD,
    shipping_cost: Some(50.into()),
    surcharge_amount: Some(20.into()),
    tax_on_surcharge: Some(2.into()),
    tax_details: Some(tax_details),
    // ... other fields
};

let net_amount = amount_details.calculate_net_amount();
```

## Integration with Other Crates

The `hyperswitch_domain_models` crate interacts with several other crates in the Hyperswitch ecosystem:

1. **api_models**: Consumes API request/response models and provides conversion to domain models
2. **diesel_models**: Provides conversion from domain models to database models
3. **common_enums**: Uses shared enumerations for status codes, currencies, etc.
4. **common_utils**: Leverages utility functions for encryption, date handling, etc.
5. **masking**: Integrates for PII masking and secure data handling
6. **router**: Consumes domain models to implement payment flows and business logic

## Performance Considerations

The crate includes several optimizations:

- **Lazy Loading**: Remote references can be loaded on demand
- **Efficient Serialization**: Custom serialization for performance-critical structs
- **Memory Efficiency**: Uses appropriate data structures for memory optimization

## Thread Safety and Async Support

The crate is designed for use in an async context:

- **Send + Sync Types**: All public types implement Send and Sync for thread safety
- **Async Methods**: Methods that may involve I/O are async
- **Tokio Integration**: Built on Tokio for async runtime compatibility

## Conclusion

The `hyperswitch_domain_models` crate is a critical component of the Hyperswitch platform, serving as the domain layer that encapsulates business logic and provides a bridge between the API and database layers. It enforces business rules, manages data transformations, and ensures data integrity throughout the payment processing lifecycle.
