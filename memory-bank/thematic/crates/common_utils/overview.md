# Common Utils Crate Overview

The `common_utils` crate provides a comprehensive set of utility functions, traits, and types that are shared across the Hyperswitch payment orchestration platform. It serves as a foundational crate that offers common functionality required by multiple components of the system, promoting code reuse and maintaining consistency across the platform.

## Purpose

The `common_utils` crate is responsible for:

1. Providing cryptographic functions for security operations
2. Managing error handling with custom error types and context
3. Offering date and time utilities for temporal operations
4. Generating secure unique identifiers for various entities
5. Handling Personal Identifiable Information (PII) securely
6. Providing serialization and deserialization helpers
7. Implementing common validation functions
8. Offering type conversions and extensions

## Key Modules

### crypto.rs

The cryptography module provides secure implementations of cryptographic algorithms:

- **Cryptographic Traits**: SignMessage, VerifySignature, EncodeMessage, DecodeMessage
- **Algorithm Implementations**: 
  - HMAC (SHA1, SHA256, SHA512)
  - Secure hashing (SHA256, SHA512, MD5, Blake3)
  - AES-GCM-256 for encryption and decryption
  - Triple DES (optional feature)
- **Random Generation**: Utilities for generating cryptographically secure random strings and bytes
- **PII Encryption**: Encryptable wrapper type for securing sensitive data

### errors.rs

Provides standardized error handling across the platform:

- **CustomResult**: A wrapper around `error_stack::Result` for consistent error reporting
- **Error Types**: Specialized error types for different operations (ParsingError, ValidationError, CryptoError, etc.)
- **Error Switching**: Utilities for converting between error contexts while preserving error information
- **Error Propagation**: Mechanisms for adding context to errors as they propagate through the system

### date_time

Date and time utilities for handling temporal data:

- **Time Formatting**: Methods for formatting dates in various formats (YYYYMMDDHHmmss, YYYYMMDD, etc.)
- **Current Time**: Utilities for getting the current time in UTC
- **Custom Serialization**: Serialization strategies for different date formats
- **Time Measurement**: Utilities for measuring execution time of code blocks (with async support)

### id_type

Specialized ID types and generation functions:

- **ID Generation**: Functions for generating unique identifiers with specific formats
- **Nanoid-based IDs**: Secure random ID generation with configurable prefixes and lengths
- **Time-ordered IDs**: Generation of UUIDs that maintain time ordering (using UUID v7)
- **Entity-specific IDs**: Specialized ID types for customers, organizations, merchants, etc.

### pii.rs

Personal Identifiable Information handling:

- **PII Masking**: Strategies for masking sensitive information when logging or displaying
- **Encryption Strategy**: Traits and implementations for securing PII data
- **PII Types**: Types for handling email, phone numbers, and other sensitive information

### validation.rs

Data validation utilities:

- **Validation Functions**: Methods for validating common formats (email, phone, URLs)
- **Field Validation**: Utilities for validating fields in requests and models
- **Constraint Enforcement**: Functions for enforcing data constraints

### consts.rs

Constants used throughout the platform:

- **ID Generation**: Constants for ID length, alphabets for nanoid generation
- **Base64 Encoding**: Engines for Base64 encoding and decoding
- **Defaults**: Default values for various operations
- **Common Values**: Frequently used values like currency precisions

### custom_serde

Custom serialization and deserialization implementations:

- **Base64 Serialization**: Serialization of binary data as Base64
- **Date Format Serializers**: Serialization for various date formats
- **Type Conversions**: Serialization helpers for type conversions

## Core Features

### Secure Cryptography

The crate provides a wide range of cryptographic functions that ensure the security of sensitive data:

- **Message Signing**: Sign messages using HMAC with different algorithms (SHA1, SHA256, SHA512)
- **Signature Verification**: Verify message signatures to ensure authenticity
- **Secure Encryption**: Encrypt data using AES-GCM-256 for strong security
- **Secure Hash Functions**: Generate secure hashes using SHA256, SHA512, MD5, and Blake3
- **Random Generation**: Generate cryptographically secure random strings and bytes for keys and tokens

### Robust Error Handling

The error handling in `common_utils` is designed to provide comprehensive context and type safety:

- **Error Context**: Errors include context about where they occurred and what caused them
- **Error Stack**: Uses the `error-stack` crate to maintain a stack of errors for detailed debugging
- **Type-safe Errors**: Different error types for different kinds of errors (parsing, validation, crypto, etc.)
- **Error Conversion**: Utilities for converting between error types while preserving context

### ID Generation

The crate provides utilities for generating various types of secure unique identifiers:

- **Nanoid Generation**: Generate secure random IDs with configurable length and alphabet
- **Prefixed IDs**: Generate IDs with specific prefixes (e.g., "cus_" for customers)
- **Time-ordered IDs**: Generate UUIDs that maintain time ordering for sortable IDs
- **Type-safe IDs**: Strongly typed ID wrappers for different entity types (customer, organization, etc.)

### Date and Time Utilities

Comprehensive utilities for working with dates and times:

- **UTC Time**: Functions for getting the current time in UTC
- **Formatting**: Convert dates to various formats (YYYYMMDD, YYYYMMDDHHmmss, etc.)
- **Parsing**: Parse dates from different formats
- **Serialization**: Custom serialization strategies for different date formats
- **Time Measurement**: Measure execution time of code blocks, including async code

### PII Protection

Utilities for securely handling personal identifiable information:

- **Masking**: Hide sensitive information when logging or displaying
- **Encryption**: Encrypt PII data for storage
- **Serialization Control**: Control how PII is serialized in outputs
- **Type Safety**: Specific types for different kinds of PII (email, phone, etc.)

## Usage Examples

### Cryptography

```rust
use common_utils::crypto::{SignMessage, VerifySignature, HmacSha256};

// Sign a message
let message = r#"{"type":"payment_intent"}"#.as_bytes();
let secret = "hmac_secret_1234".as_bytes();

let signature = HmacSha256.sign_message(secret, message)?;

// Verify a signature
let is_valid = HmacSha256.verify_signature(secret, &signature, message)?;
assert!(is_valid);

// Generate a random secure string
let random_key = common_utils::crypto::generate_cryptographically_secure_random_string(32);
```

### ID Generation

```rust
use common_utils::{generate_id, generate_id_with_default_len};

// Generate an ID with custom length
let payment_id = generate_id(10, "pay"); // pay_xxxxxxxxxx

// Generate an ID with default length
let customer_id = generate_id_with_default_len("cus"); // cus_xxxxxxxxxxxxxxxxx

// Generate a time-ordered ID
let order_id = common_utils::generate_time_ordered_id("ord"); // ord_xxxxxxxxxxxxxxxxxxxxxxxx

// Generate a typed customer ID
let typed_customer_id = common_utils::generate_customer_id_of_default_length();
```

### Date and Time

```rust
use common_utils::date_time::{now, format_date, DateFormat};

// Get current time
let current_time = now();

// Format date as YYYYMMDDHHmmss
let formatted = format_date(current_time, DateFormat::YYYYMMDDHHmmss)?; // "20230215081132"

// Format date as ISO8601 with milliseconds
let iso_date = common_utils::date_time::date_as_yyyymmddthhmmssmmmz()?; // "2023-02-15T13:33:18.898Z"

// Measure execution time of async code
let (result, time_ms) = common_utils::date_time::time_it(|| async {
    // Some async operation
    "result"
}).await;
println!("Operation took {}ms", time_ms);
```

### Error Handling

```rust
use common_utils::errors::{CustomResult, ParsingError};
use error_stack::{Report, ResultExt};

// Return a custom result
fn parse_value(input: &str) -> CustomResult<i32, ParsingError> {
    input
        .parse::<i32>()
        .map_err(|_| Report::new(ParsingError::EnumParseFailure("i32")))
        .attach_printable_lazy(|| format!("Failed to parse '{}' as i32", input))
}

// Using error switching
use common_utils::errors::{ReportSwitchExt, ValidationError, ErrorSwitchFrom};

// Implement switching between error types
impl ErrorSwitchFrom<ParsingError> for ValidationError {
    fn switch_from(error: &ParsingError) -> Self {
        match error {
            ParsingError::EnumParseFailure(name) => ValidationError::InvalidValue { 
                message: format!("Invalid enum value: {}", name) 
            },
            _ => ValidationError::InvalidValue { 
                message: "Invalid value".to_string() 
            },
        }
    }
}

// Switch error context
let result = parse_value("abc").switch();
```

## Integration with Other Crates

The `common_utils` crate is used extensively throughout the Hyperswitch ecosystem:

1. **router**: Uses common utilities for ID generation, error handling, cryptography, and more
2. **storage_impl**: Uses error handling, ID generation, and database connection utilities
3. **api_models**: Uses validation utilities, error types, and custom serialization
4. **hyperswitch_connectors**: Uses cryptographic functions for signing requests and verifying responses
5. **redis_interface**: Uses error handling, serialization, and ID generation

## Performance Considerations

The crate includes several performance optimizations:

- **Zero-cost Abstractions**: Uses Rust's type system to provide abstractions with no runtime cost
- **Efficient ID Generation**: Uses optimized algorithms for ID generation
- **Memory Efficiency**: Avoids unnecessary allocations and copies
- **Caching**: Implements caching where appropriate to avoid redundant operations
- **Async Support**: Provides async versions of functions where blocking would be inefficient

## Thread Safety and Async Support

The crate is designed for concurrent usage:

- **Thread Safety**: All public types are thread-safe (Send + Sync where applicable)
- **Async Support**: Provides async versions of functions that might block
- **No Global State**: Avoids global state that could cause concurrency issues
- **Immutable Data**: Prefers immutable data structures to avoid synchronization overhead

## Conclusion

The `common_utils` crate is a foundational component of the Hyperswitch platform. It provides a rich set of utilities that ensure security, consistency, and efficiency across the platform. The crate's emphasis on type safety, error handling, and cryptographic security makes it an essential part of the platform's architecture.
