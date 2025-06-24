# Cards Crate Overview

## Purpose and Overview

The `cards` crate provides specialized types and validation utilities for securely handling payment card information within the Hyperswitch payment orchestration platform. It offers robust validation, secure storage, and appropriate masking of sensitive card data, ensuring PCI DSS compliance while making card-related operations safer and more reliable.

This crate acts as a foundational component for any functionality in Hyperswitch that involves card data, providing strong typing around card numbers, security codes, and expiration dates with built-in validation and security measures.

## Key Components

### Card Data Types

1. **CardNumber**: A type-safe wrapper for credit/debit card numbers that:
   - Validates input using the Luhn algorithm
   - Enforces ISO-standard length restrictions (8-19 digits)
   - Implements secure masking (showing only first 6 digits)
   - Provides secure methods to access specific parts (BIN, last 4 digits)

2. **NetworkToken**: Similar to CardNumber but specifically for handling network tokens, with the same validation and security features.

3. **CardSecurityCode**: A secure wrapper for CVV/CVC codes that validates the input is a legitimate security code value.

4. **CardExpiration**: Composite type for card expiration dates that:
   - Contains month and year components
   - Validates expiration hasn't already occurred
   - Handles formatting for different display requirements

### Validation Utilities

1. **Luhn Algorithm Implementation**: Validates card numbers through the industry-standard checksum formula.

2. **Card Network Detection**: Identifies card networks (Visa, Mastercard, etc.) based on regex patterns for BIN ranges.

3. **Co-badged Card Detection**: Detects when a card number matches the patterns for multiple card networks.

4. **Secure Validation**: Implements validation in a way that doesn't compromise security of the card data.

## Code Structure

```
/crates/cards/
├── src/
│   ├── lib.rs             # Core types and implementation for CardExpiration and CardSecurityCode
│   └── validate.rs        # Card number validation logic, Luhn algorithm, and masking strategy
└── tests/
    └── basic.rs           # Unit tests
```

### Key Files and Their Roles

- **lib.rs**: Defines the main card-related types (CardSecurityCode, CardExpirationMonth, CardExpirationYear, CardExpiration) with validation logic.

- **validate.rs**: Implements the CardNumber and NetworkToken types, along with validation utilities like Luhn algorithm checking and card network detection.

## Security Features

1. **Strong Secret Wrapping**: All sensitive data is wrapped in `StrongSecret` from the `masking` crate, preventing accidental leaks in logs and error messages.

2. **Custom Masking Strategy**: The `CardNumberStrategy` defines how card numbers should be masked in various contexts (showing only first 6 digits).

3. **Secure Deserialization**: Custom deserialization with validation ensures invalid card data cannot enter the system.

4. **PCI DSS Compliance Support**: Implementation follows best practices for handling payment card data.

## Usage Examples

### Creating and Validating Card Numbers

```rust
use cards::CardNumber;
use std::str::FromStr;

// Parse and validate a card number string
let card_number = CardNumber::from_str("4111 1111 1111 1111")?;

// Get masked representation for display/logging (shows "411111**********")
println!("Card: {}", card_number);

// Get card BIN for routing or validation
let bin = card_number.get_card_isin(); // "411111"
let extended_bin = card_number.get_extended_card_bin(); // "41111111"

// Get last 4 digits for receipt/confirmation
let last4 = card_number.get_last4(); // "1111"

// Check if it's a co-badged card (matches multiple networks)
let is_cobadged = card_number.is_cobadged_card()?;
```

### Working with Card Expiration

```rust
use cards::CardExpiration;

// Create expiration from month and year
let expiration = CardExpiration::try_from((12_u8, 2025_u16))?;

// Check if card is expired
if expiration.is_expired()? {
    // Handle expired card
}

// Format for display
println!("Expires: {}/{}", 
    expiration.get_month().two_digits(),
    expiration.get_year().two_digits()); // "12/25"
```

## Integration with Other Crates

The `cards` crate integrates with several other crates in the Hyperswitch ecosystem:

1. **masking**: Leverages the `masking` crate for secure handling of sensitive data and appropriate masking in logs.

2. **common_utils**: Uses error types and utilities from `common_utils` for standardized error handling.

3. **router_env**: Integrates with logging and environment detection for conditional behavior based on deployment environment.

4. **router**: Consumed by the `router` crate for validating and handling card information in payment flows.

5. **hyperswitch_domain_models**: Used by domain models when modeling payment methods and card details.

## Performance Considerations

1. **Minimal Allocations**: Card validation is designed to minimize allocations and copying of sensitive data.

2. **Efficient Validation**: The Luhn algorithm and other validations are implemented with performance in mind.

3. **Zero-Copy Where Possible**: Operations like extracting card BIN or last 4 digits don't create unnecessary copies of the full card number.

## Configuration Options

The crate provides special handling for test environments:

- In development and sandbox environments, certain test card numbers are accepted even without passing validation
- In production, these test card bypasses are disabled

## Best Practices

When using the `cards` crate:

1. Always use the provided types rather than handling raw card data as strings

2. Leverage the secure accessors (like `get_last4()`) rather than extracting and manipulating the raw card number

3. Be aware of the masking strategy when logging card-related information

4. Use the validation functions to verify card data at system boundaries

## Links to Source Code

- [crates/cards/src/lib.rs](/Users/arunraj/github/hyperswitch/crates/cards/src/lib.rs) - Core type definitions and implementations
- [crates/cards/src/validate.rs](/Users/arunraj/github/hyperswitch/crates/cards/src/validate.rs) - Card validation logic

---

*Last Updated: 2025-05-20*  
*Maintainers: Hyperswitch Core Team*
