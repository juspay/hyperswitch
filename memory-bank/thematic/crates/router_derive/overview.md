# Router Derive Crate

## Purpose and Overview

The `router_derive` crate provides a collection of procedural macros and attribute macros that simplify common patterns used throughout the Hyperswitch codebase. These macros reduce boilerplate code, automate trait implementations, and provide consistent error handling patterns, enabling more maintainable and concise code.

This crate is a core utility in the Hyperswitch ecosystem, providing code generation capabilities that handle complex patterns like database interactions, API error responses, permission management, and more.

## Key Components

The crate offers the following procedural and attribute macros:

### Core Utility Macros

1. **`DebugAsDisplay`** - Derives the `Display` implementation for a type by reusing its `Debug` implementation.

2. **`Setter`** - Generates setter methods for struct fields, following a fluent builder pattern.

3. **`FlatStruct`** - Converts nested struct hierarchies into flattened key-value pairs, useful for configuration management and serialization.

### Database-Related Macros

4. **`DieselEnum` / `DieselEnumText`** - Derives the boilerplate code required for using Rust enums with Diesel and PostgreSQL databases.

5. **`diesel_enum`** - An attribute macro that works with `DieselEnum` to specify storage types (either as text or as a database enum).

### API and Error Handling Macros

6. **`ApiError`** - Provides a standardized serialization for API error responses, ensuring consistency across the API surface.

7. **`TryGetEnumVariant`** - Generates helper methods to safely extract values from enum variants, with proper error handling.

### Payment-Specific Macros

8. **`PaymentOperation`** - Derives the `Operation` trait implementation for payment-related structs, providing a standardized interface for payment operations.

### Schema and Validation Macros

9. **`PolymorphicSchema`** - Generates different schema structs with the ability to mark certain fields as mandatory for specific schemas, useful for API request validation.

10. **`ConfigValidate`** - Implements validation logic for configuration structs to ensure required values are present.

### Security and Permission Macros

11. **`generate_permissions!`** - A declarative macro that generates permission enums and implementations based on specified resources, scopes, and entity types.

12. **`ToEncryption`** - Derives functionality for converting sensitive data to and from encrypted formats.

## Code Structure

```
/crates/router_derive/
├── src/
│   ├── lib.rs                 # Entry point with macro definitions
│   ├── macros.rs              # Macro implementation coordination
│   └── macros/                # Individual macro implementations
│       ├── api_error.rs       # ApiError macro implementation
│       ├── diesel.rs          # Diesel-related macros
│       ├── generate_permissions.rs 
│       ├── generate_schema.rs
│       ├── helpers.rs         # Shared helper functions
│       ├── misc.rs            # Miscellaneous utility macros
│       ├── operation.rs       # PaymentOperation macro
│       ├── to_encryptable.rs  # Encryption-related macros
│       └── try_get_enum.rs    # Enum variant extraction
```

## Usage Examples

### Example 1: Debug as Display

```rust
use router_derive::DebugAsDisplay;

#[derive(Debug, DebugAsDisplay)]
struct Point {
    x: f32,
    y: f32,
}

// Automatically implements Display using the Debug implementation
println!("{}", Point { x: 1.0, y: 2.0 }); // Output: Point { x: 1.0, y: 2.0 }
```

### Example 2: Database Enum Support

```rust
use router_derive::{diesel_enum, DieselEnum};
use strum::{Display, EnumString};

#[derive(Display, EnumString, Debug)]
#[diesel_enum(storage_type = "db_enum")]
enum PaymentStatus {
    Pending,
    Authorized,
    Captured,
    Failed,
}

// Generated code enables seamless integration with Diesel and PostgreSQL
// without manually implementing conversions
```

### Example 3: API Error Handling

```rust
use router_derive::ApiError;

#[derive(Clone, Debug, serde::Serialize)]
enum ErrorType {
    ValidationError,
    DatabaseError,
    ConnectorError,
}

#[derive(Debug, ApiError)]
#[error(error_type_enum = ErrorType)]
enum ApiErrorResponse {
    #[error(
        error_type = ErrorType::ValidationError,
        code = "E001",
        message = "Invalid payment data: {field_name}"
    )]
    InvalidPaymentData { field_name: String },
    
    #[error(
        error_type = ErrorType::ConnectorError,
        code = "E002",
        message = "Connector error occurred"
    )]
    ConnectorError,
}

// Standardized JSON error format is automatically generated
```

### Example 4: Polymorphic Schemas

```rust
use router_derive::PolymorphicSchema;

#[derive(PolymorphicSchema)]
#[generate_schemas(PaymentsCreateRequest, PaymentsConfirmRequest)]
struct PaymentsRequest {
    #[mandatory_in(PaymentsCreateRequest = u64)]
    amount: Option<u64>,
    
    #[mandatory_in(PaymentsCreateRequest = String)]
    currency: Option<String>,
    
    payment_method: String,
}

// Generates two separate request schemas with different validation rules
```

## Integration with Other Crates

The `router_derive` crate is primarily used by the following crates:

1. **`router`** - The core crate uses most of these macros for its API endpoints, error handling, and payment operations.

2. **`hyperswitch_domain_models`** - Uses database-related macros for domain model definitions.

3. **`api_models`** - Utilizes the schema generation macros for API request and response models.

4. **`common_utils`** - May use utility macros like `DebugAsDisplay` for common functionality.

## Dependencies

Main dependencies include:

- **`proc-macro2`**, **`quote`**, **`syn`** - Core crates for procedural macro development
- **`indexmap`** - For ordered map implementations used in some macros
- **`serde_json`** - For JSON serialization in certain macros
- **`strum`** - For enum string conversions

Dev dependencies include:

- **`diesel`** - For testing database-related macros
- **`error-stack`** - For error handling in tests
- **`serde`** - For serialization testing
- **`utoipa`** - For OpenAPI schema generation in tests
- **`common_utils`** - For testing utility integrations

## Links to Source Code

- [crates/router_derive/src/lib.rs](/Users/arunraj/github/hyperswitch/crates/router_derive/src/lib.rs) - Main entry point with macro definitions
- [crates/router_derive/src/macros/api_error.rs](/Users/arunraj/github/hyperswitch/crates/router_derive/src/macros/api_error.rs) - API error handling implementation
- [crates/router_derive/src/macros/diesel.rs](/Users/arunraj/github/hyperswitch/crates/router_derive/src/macros/diesel.rs) - Database enum handling
- [crates/router_derive/src/macros/operation.rs](/Users/arunraj/github/hyperswitch/crates/router_derive/src/macros/operation.rs) - Payment operation trait implementation

## Best Practices

When using the macros from this crate:

1. Always read the documentation for each macro to understand the generated code
2. For database enums, ensure the enum implements `ToString` and `FromStr` traits
3. With `ApiError`, ensure all error variants have appropriate error codes and messages
4. Be cautious with `PolymorphicSchema` as it generates multiple structs at compile time

---

*Last Updated: 2025-05-20*  
*Maintainers: Hyperswitch Core Team*
