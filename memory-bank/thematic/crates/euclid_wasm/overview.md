# Euclid WASM Overview

The `euclid_wasm` crate provides WebAssembly (WASM) bindings for the Euclid Domain-Specific Language (DSL), enabling browser-based interactions with Hyperswitch's payment routing rules engine. This document outlines its purpose, architecture, and usage within the Hyperswitch ecosystem.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `euclid_wasm` crate is responsible for:

1. Exposing Euclid DSL functionality to web/JavaScript environments through WebAssembly
2. Enabling browser-based creation, validation, and execution of payment routing rules
3. Providing JavaScript interfaces for working with routing conditions, connectors, and rule analysis
4. Supporting frontend tools for rule editing and visualization
5. Bridging JavaScript and Rust type systems for seamless data exchange

## Key Modules

The `euclid_wasm` crate is organized into the following modules:

- **lib.rs**: Main entry point with WebAssembly exported functions
- **types.rs**: Type definitions specific to the WASM interface
- **utils.rs**: Utility functions for error handling and JavaScript interoperability

## Core Features

### JavaScript Binding Layer

The crate uses `wasm-bindgen` to expose Rust functions to JavaScript:

- **Type Conversions**: Seamless conversion between Rust and JavaScript types
- **Error Handling**: Proper error propagation from Rust to JavaScript
- **Memory Management**: Efficient memory handling between WASM and JavaScript

### Routing Rule Management

Exposes key functionality for working with payment routing rules:

- **Rule Validation**: Validating rules against constraints and connector capabilities
- **Rule Execution**: Running routing rules against payment data
- **Rule Analysis**: Analyzing rule structures for consistency and correctness
- **Connector Selection**: Determining valid connectors for specific rules

### Currency Conversion

Provides currency conversion capabilities to the frontend:

- **Forex Data Management**: Functions to manage exchange rate data
- **Currency Conversion**: Converting amounts between different currencies
- **Rate Updates**: Ability to update exchange rates at runtime

### Metadata Access

Exposes metadata about payment routing components:

- **Connector Information**: Lists available payment connectors
- **Key Information**: Provides details about available routing rule keys
- **Variant Values**: Lists possible values for different routing condition types
- **Type Information**: Returns type information for different keys
- **Descriptive Metadata**: Provides detailed descriptions for keys and values

## Public Interface

The crate exposes numerous JavaScript-callable functions through the `wasm-bindgen` annotations. Key functions include:

### Rule Management

```javascript
// Analyze a routing program for errors
analyzeProgram(program)

// Execute a routing program with specific input
runProgram(program, input)

// Get valid connectors for a specific rule
getValidConnectorsForRule(rule)
```

### Currency Conversion

```javascript
// Set forex data for currency conversion
setForexData(forexData)

// Convert currency values
convertCurrency(amount, fromCurrency, toCurrency)
```

### Metadata Functions

```javascript
// Get all available connectors
getAllConnectors()

// Get all available keys for rules
getAllKeys()

// Get type information for a specific key
getKeyType(key)

// Get possible values for a key variant
getVariantValues(key)

// Get descriptions and categories for keys
getDescriptionCategory()
```

### Connector Configuration

```javascript
// Get configuration for payment connectors
getConnectorConfig(connectorKey)

// Get configuration for payout connectors
getPayoutConnectorConfig(connectorKey)

// Get configuration for authentication connectors
getAuthenticationConnectorConfig(connectorKey)
```

## Feature Flags

The crate supports several feature flags for customization:

- **default**: Includes payouts functionality
- **release**: Production-ready configuration
- **dummy_connector**: Enables dummy connector for testing
- **production**: Uses production connector configurations
- **sandbox**: Uses sandbox connector configurations
- **payouts**: Enables payout functionality
- **v1/v2**: API version compatibility

## Integration with Other Crates

The `euclid_wasm` crate integrates with several other parts of the Hyperswitch ecosystem:

1. **euclid**: Core DSL implementation that this crate exposes to JavaScript
2. **hyperswitch_constraint_graph**: Used for rule validation and connector compatibility
3. **connector_configs**: Provides connector configuration details
4. **api_models**: Defines shared data models between backend and frontend
5. **kgraph_utils**: Used for knowledge graph operations and rule verification
6. **currency_conversion**: Provides currency conversion functionality

## Usage Examples

### Basic Rule Validation

```javascript
// JavaScript code using the WASM module
import * as wasm from 'euclid_wasm';

const rule = {
  name: "high_value_payment",
  condition: {
    operator: "AND",
    conditions: [
      { key: "PaymentAmount", operator: "GREATER_THAN", value: 1000 },
      { key: "PaymentCurrency", operator: "EQUALS", value: "USD" }
    ]
  },
  connector_selection: {
    connector: "stripe",
    priority: 1
  }
};

// Analyze rule for errors
try {
  wasm.analyzeProgram(rule);
  console.log("Rule is valid");
  
  // Get valid connectors for this rule
  const validConnectors = wasm.getValidConnectorsForRule(rule);
  console.log("Valid connectors:", validConnectors);
} catch (error) {
  console.error("Rule validation failed:", error);
}
```

### Currency Conversion

```javascript
// JavaScript code for currency conversion
import * as wasm from 'euclid_wasm';

// Set forex data
const forexData = {
  base_currency: "USD",
  conversions: {
    "EUR": 0.92,
    "GBP": 0.78,
    "JPY": 149.5
  }
};

wasm.setForexData(forexData);

// Convert 100 USD to EUR
const eurAmount = wasm.convertCurrency(100, "USD", "EUR");
console.log("100 USD = " + eurAmount + " EUR");
```

### Getting Metadata

```javascript
// JavaScript code to get metadata
import * as wasm from 'euclid_wasm';

// Get all available connectors
const connectors = wasm.getAllConnectors();
console.log("Available connectors:", connectors);

// Get all available keys for conditions
const keys = wasm.getAllKeys();
console.log("Available keys:", keys);

// Get possible values for a specific key
const paymentMethodValues = wasm.getVariantValues("PaymentMethod");
console.log("Payment method values:", paymentMethodValues);

// Get descriptions and categories
const categories = wasm.getDescriptionCategory();
console.log("Key categories:", categories);
```

## Implementation Details

### JavaScript/Rust Type Conversion

The crate uses `serde_wasm_bindgen` to handle type conversion:

- **JavaScript to Rust**: Converts JavaScript objects to Rust structs
- **Rust to JavaScript**: Converts Rust data back to JavaScript objects
- **Error Handling**: Properly converts Rust errors to JavaScript exceptions

### Data Seeding and State Management

The crate maintains some global state:

- **Knowledge Graph**: Stores rule validation data in `SEED_DATA`
- **Forex Data**: Stores currency conversion rates in `SEED_FOREX`
- **Initialization**: Provides functions to seed data from JavaScript

### Security Considerations

- **Type Validation**: All JavaScript inputs are validated before use
- **Error Handling**: Proper error propagation to prevent undefined behavior
- **Feature Flags**: Controls which functionality is exposed based on requirements

## Testing Strategy

Testing for the WASM crate involves:

1. **Unit Tests**: Testing individual functions in isolation
2. **Integration Tests**: Testing with actual JavaScript code
3. **Browser Tests**: Ensuring compatibility in different browser environments
4. **End-to-End Tests**: Testing complete user flows with the frontend

## Browser Compatibility

The crate is designed to work with modern browsers that support WebAssembly:

- **Chrome/Edge**: Version 57 and above
- **Firefox**: Version 52 and above
- **Safari**: Version 11 and above
- **Node.js**: Version 8 and above

## Performance Considerations

- **Binary Size**: The WASM binary is optimized for size to minimize download time
- **Memory Usage**: Careful management of memory between JavaScript and WASM
- **Computation Efficiency**: Complex operations happen in Rust for better performance

## Conclusion

The `euclid_wasm` crate serves as a critical bridge between the browser-based frontend and the Rust-based Euclid DSL, enabling sophisticated payment routing rule creation and management directly in web interfaces. It leverages WebAssembly to provide the performance of Rust with the accessibility of JavaScript, making it a key component in Hyperswitch's web-based administration tools.

## See Also

- [Euclid Overview](../euclid/overview.md)
- [Euclid Macros Overview](../euclid_macros/overview.md)
- [Hyperswitch Constraint Graph Overview](../hyperswitch_constraint_graph/overview.md)
