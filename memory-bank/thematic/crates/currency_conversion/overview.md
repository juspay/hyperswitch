# Currency Conversion Crate Overview

The `currency_conversion` crate provides a robust and accurate currency conversion framework for the Hyperswitch payment orchestration platform. It enables precise conversion of monetary amounts between different currencies, handling the complexities of exchange rates, decimal precision, and conversion factors.

## Purpose

The `currency_conversion` crate is responsible for:

1. Converting payment amounts between different currencies
2. Managing exchange rates with a base currency model
3. Handling decimal arithmetic with proper precision
4. Providing ISO currency standard integration
5. Ensuring accurate conversions for financial transactions
6. Managing conversion errors gracefully

## Key Modules

### conversion.rs

The conversion module provides the primary functionality:

- **Convert Function**: Main entry point for currency conversions
- **Path-based Conversion**: Handles direct, forward, and backward conversion paths
- **Money Integration**: Works with the rusty_money library for monetary values
- **Error Handling**: Proper error management for failed conversions

### types.rs

Defines the data structures used in currency conversion:

- **ExchangeRates**: Stores the base currency and conversion factors for all supported currencies
- **CurrencyFactors**: Stores to/from factors for conversion between currencies
- **Currency Matching**: Maps between Currency enum and ISO currency implementations
- **Forward/Backward Conversion**: Methods for converting to and from base currency

### error.rs

Implements error handling for currency conversion operations:

- **ConversionNotSupported**: Error for unsupported currency conversions
- **DecimalMultiplicationFailed**: Error for arithmetic failures during conversion
- **Serialization Support**: Errors can be serialized for API responses

## Core Features

### Base Currency Conversion Model

The crate employs a base currency intermediary model for conversions:

- **Base Currency Hub**: All conversions route through a base currency (typically USD)
- **Two-factor Model**: Each currency has separate factors for to/from base currency
- **Intermediary Conversion**: When converting between two non-base currencies, conversion flows through base currency

### Exchange Rate Management

Comprehensive management of exchange rates:

- **Rate Storage**: Efficient storage of conversion factors
- **Exchange Rate Lookup**: Fast lookup of conversion rates
- **Currency Pairing**: Handles all currency pair combinations

### Precise Decimal Arithmetic

Handles currency arithmetic with appropriate precision:

- **Decimal Type**: Uses Rust's decimal crate for accurate financial calculations
- **Multiplication Safeguards**: Checks for overflow or underflow conditions
- **Minor Unit Handling**: Works with amounts in minor units (cents, pence, etc.)
- **Precision Control**: Maintains appropriate decimal precision

### Currency Standards Support

Integration with ISO currency standards:

- **ISO 4217 Support**: Full support for ISO 4217 currency codes
- **Minor Units**: Proper handling of currency-specific decimal places
- **Currency Matching**: Comprehensive mapping to standard currency implementations

## Usage Examples

### Basic Currency Conversion

```rust
use common_enums::Currency;
use currency_conversion::{
    conversion::convert,
    types::{CurrencyFactors, ExchangeRates},
};
use rust_decimal::Decimal;
use std::collections::HashMap;

// Set up exchange rates with USD as base currency
let mut conversion_map: HashMap<Currency, CurrencyFactors> = HashMap::new();

// Configure EUR/USD conversion factors
let eur_factors = CurrencyFactors::new(
    Decimal::new(10762, 4),  // 1 USD = 1.0762 EUR (to_factor)
    Decimal::new(9292, 5),   // 1 EUR = 0.9292 USD (from_factor)
);
conversion_map.insert(Currency::EUR, eur_factors);

// Configure GBP/USD conversion factors
let gbp_factors = CurrencyFactors::new(
    Decimal::new(8451, 4),   // 1 USD = 0.8451 GBP (to_factor)
    Decimal::new(1183, 4),   // 1 GBP = 1.183 USD (from_factor)
);
conversion_map.insert(Currency::GBP, gbp_factors);

// Create exchange rates object
let exchange_rates = ExchangeRates::new(Currency::USD, conversion_map);

// Convert 100 EUR to GBP
let amount_in_eur = 10000; // 100.00 EUR in minor units
let amount_in_gbp = convert(&exchange_rates, Currency::EUR, Currency::GBP, amount_in_eur)
    .expect("Currency conversion failed");

println!("100.00 EUR = {:.2} GBP", amount_in_gbp / 100);
```

### Forward Conversion (to Base Currency)

```rust
use common_enums::Currency;
use currency_conversion::types::{CurrencyFactors, ExchangeRates};
use rust_decimal::Decimal;
use std::collections::HashMap;

// Set up exchange rates
let mut conversion_map: HashMap<Currency, CurrencyFactors> = HashMap::new();
let inr_factors = CurrencyFactors::new(
    Decimal::new(8231, 2),   // 1 USD = 82.31 INR (to_factor)
    Decimal::new(1215, 5),   // 1 INR = 0.01215 USD (from_factor)
);
conversion_map.insert(Currency::INR, inr_factors);

let exchange_rates = ExchangeRates::new(Currency::USD, conversion_map);

// Convert 1000 INR to USD (forward conversion)
let amount_in_inr = Decimal::new(1000, 0);
let amount_in_usd = exchange_rates.forward_conversion(amount_in_inr, Currency::INR)
    .expect("Forward conversion failed");

println!("1000 INR = {:.2} USD", amount_in_usd);
```

### Backward Conversion (from Base Currency)

```rust
use common_enums::Currency;
use currency_conversion::types::{CurrencyFactors, ExchangeRates};
use rust_decimal::Decimal;
use std::collections::HashMap;

// Set up exchange rates
let mut conversion_map: HashMap<Currency, CurrencyFactors> = HashMap::new();
let jpy_factors = CurrencyFactors::new(
    Decimal::new(1503, 2),   // 1 USD = 150.3 JPY (to_factor)
    Decimal::new(665, 5),    // 1 JPY = 0.00665 USD (from_factor)
);
conversion_map.insert(Currency::JPY, jpy_factors);

let exchange_rates = ExchangeRates::new(Currency::USD, conversion_map);

// Convert 50 USD to JPY (backward conversion)
let amount_in_usd = Decimal::new(50, 0);
let amount_in_jpy = exchange_rates.backward_conversion(amount_in_usd, Currency::JPY)
    .expect("Backward conversion failed");

println!("50 USD = {:.0} JPY", amount_in_jpy);
```

### Error Handling

```rust
use common_enums::Currency;
use currency_conversion::{
    conversion::convert,
    error::CurrencyConversionError,
    types::{CurrencyFactors, ExchangeRates},
};
use rust_decimal::Decimal;
use std::collections::HashMap;

// Set up exchange rates with limited currencies
let mut conversion_map: HashMap<Currency, CurrencyFactors> = HashMap::new();
let eur_factors = CurrencyFactors::new(
    Decimal::new(1076, 3),
    Decimal::new(929, 3),
);
conversion_map.insert(Currency::EUR, eur_factors);

let exchange_rates = ExchangeRates::new(Currency::USD, conversion_map);

// Try converting from an unsupported currency
let result = convert(&exchange_rates, Currency::AUD, Currency::EUR, 100);

match result {
    Ok(amount) => println!("Converted amount: {}", amount),
    Err(CurrencyConversionError::ConversionNotSupported(currency)) => {
        println!("Currency not supported: {}", currency);
    }
    Err(CurrencyConversionError::DecimalMultiplicationFailed) => {
        println!("Decimal multiplication failed");
    }
}
```

## Integration with Other Crates

The `currency_conversion` crate integrates with several other components of the Hyperswitch ecosystem:

1. **common_enums**: Uses the Currency enum to identify supported currencies
2. **rust_decimal**: Uses Decimal type for precise financial calculations
3. **rusty_money**: Integrates with the Money type for handling monetary values
4. **router**: Used by the router crate for payment amount conversions
5. **hyperswitch_domain_models**: Supports domain models for handling multicurrency payments

## Performance Considerations

The crate is designed for efficient performance while maintaining accuracy:

- **Cached Exchange Rates**: Exchange rates can be cached for repeated access
- **Minimal Allocations**: Minimizes memory allocations for high-volume applications
- **Decimal Precision**: Handles decimal arithmetic with appropriate precision for financial calculations
- **Error Checking**: Validates conversions without runtime panics
- **Efficient Lookup**: Uses HashMap for O(1) currency factor lookups

## Thread Safety and Async Support

The crate is designed for concurrent usage:

- **Thread Safety**: Core types (ExchangeRates, CurrencyFactors) implement Send and Sync
- **Immutable Data**: Structures designed for immutable sharing between threads
- **No Global State**: Avoids global state that could cause concurrency issues

## Conversion Logic Details

The conversion process follows these steps:

1. **Direct Conversions**:
   - Base to Currency: Uses backward conversion (multiply by to_factor)
   - Currency to Base: Uses forward conversion (multiply by from_factor)

2. **Cross-Currency Conversions**:
   - Convert source currency to base currency (forward)
   - Convert base currency to target currency (backward)

3. **Exchange Rate Format**:
   - to_factor: Multiplicative factor to convert from base to currency (e.g., 1 USD * 82.31 = 82.31 INR)
   - from_factor: Multiplicative factor to convert from currency to base (e.g., 1 INR * 0.01215 = 0.01215 USD)

## Conclusion

The `currency_conversion` crate is a critical component of the Hyperswitch platform's ability to handle multi-currency transactions. Its precise decimal arithmetic, efficient exchange rate management, and ISO-compliant implementation ensure that currency conversions are handled accurately and reliably throughout the payment processing pipeline.
