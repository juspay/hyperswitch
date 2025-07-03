# Knowledge Graph Utilities (kgraph_utils) Overview

The `kgraph_utils` crate provides utilities for constructing and working with knowledge graphs in the Hyperswitch ecosystem, with a primary focus on payment routing validation. This document outlines its purpose, components, and integration with other crates.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `kgraph_utils` crate is responsible for:

1. Constructing knowledge graphs that represent payment routing constraints and rules
2. Transforming merchant connector accounts into constraint graph structures
3. Validating payment method configurations against defined constraints
4. Supporting rule-based decision making for payment routing
5. Bridging between API models and graph-based constraint validation

## Key Modules

The `kgraph_utils` crate is organized into the following modules:

- **error.rs**: Error types and handling for knowledge graph operations
- **mca.rs**: Implementation for building graphs from Merchant Connector Accounts (MCA)
- **transformers.rs**: Utilities for transforming API data types into graph nodes
- **types.rs**: Type definitions for knowledge graph structures and operations

## Core Features

### Knowledge Graph Construction

The crate provides functionality for building constraint graphs:

- **MCA Graph Creation**: Converting merchant connector accounts into constraint graphs
- **Domain Identification**: Setting up logical domains for constraint validation
- **Node and Edge Creation**: Building graph structures with appropriate relationships
- **Feature-Based Compilation**: Supporting different API versions with feature flags

### Payment Method Type Transformation

Sophisticated handling of payment method types and their relationships:

- **Payment Method Mapping**: Converting API payment method types to graph node values
- **Card Network Handling**: Managing card network constraints and relationships
- **Currency and Country Filters**: Implementing geographical and currency-based rules
- **Amount Range Validation**: Supporting minimum and maximum amount constraints

### Constraint Configuration

Support for complex payment routing constraints:

- **Configuration Maps**: Building constraint configurations for different connectors
- **Filter Implementation**: Creating filters for payment methods, countries, and currencies
- **Flow Restrictions**: Implementing constraints on payment flows like capture methods
- **Aggregation Logic**: Building AND/OR logic relationships between constraints

### Validation and Testing

Built-in support for validating payment scenarios:

- **Graph Analysis**: Testing payment contexts against constraint graphs
- **Error Propagation**: Detailed error information for constraint violations
- **Test Utilities**: Support for building test scenarios and validation

## Public Interface

### Key Functions

```rust
// Create a constraint graph from merchant connector accounts
pub fn make_mca_graph(
    accts: Vec<admin_api::MerchantConnectorResponse>,
    config: &kgraph_types::CountryCurrencyFilter,
) -> Result<cgraph::ConstraintGraph<dir::DirValue>, KgraphError>
```

### Data Structures

```rust
// Payment method filter configuration
pub struct PaymentMethodFilters(pub HashMap<PaymentMethodFilterKey, CurrencyCountryFlowFilter>);

// Country and currency filter configuration
pub struct CountryCurrencyFilter {
    pub connector_configs: HashMap<RoutableConnectors, PaymentMethodFilters>,
    pub default_configs: Option<PaymentMethodFilters>,
}

// Filter configuration for specific payment flows
pub struct CurrencyCountryFlowFilter {
    pub currency: Option<HashSet<Currency>>,
    pub country: Option<HashSet<CountryAlpha2>>,
    pub not_available_flows: Option<NotAvailableFlows>,
}
```

## Usage Examples

### Building a Constraint Graph

```rust
use kgraph_utils::{mca, types};
use api_models::admin;
use common_enums::RoutableConnectors;
use std::collections::{HashMap, HashSet};

// Create merchant connector account data
let merchant_connectors = vec![
    admin::MerchantConnectorResponse {
        connector_name: "stripe".to_string(),
        payment_methods_enabled: Some(vec![
            // Payment method configurations
        ]),
        // Other connector details
        ..Default::default()
    }
];

// Set up configuration filters
let config = types::CountryCurrencyFilter {
    connector_configs: HashMap::from([(
        RoutableConnectors::Stripe,
        types::PaymentMethodFilters(HashMap::from([
            // Payment method filters
        ])),
    )]),
    default_configs: None,
};

// Build the knowledge graph
let graph = mca::make_mca_graph(merchant_connectors, &config)?;
```

### Validating a Payment Context

```rust
use euclid::{dirval, dssa::graph::AnalysisContext};
use hyperswitch_constraint_graph::{Memoization, CycleCheck};

// Create an analysis context from payment data
let context = AnalysisContext::from_dir_values([
    dirval!(Connector = Stripe),
    dirval!(PaymentMethod = Card),
    dirval!(CardType = Credit),
    dirval!(CardNetwork = Visa),
    dirval!(PaymentCurrency = USD),
    dirval!(PaymentAmount = 100),
]);

// Validate the payment context against the graph
let result = graph.key_value_analysis(
    dirval!(Connector = Stripe),
    &context,
    &mut Memoization::new(),
    &mut CycleCheck::new(),
    None,
);

// Handle the validation result
match result {
    Ok(_) => println!("Payment context is valid for Stripe"),
    Err(e) => println!("Payment context validation failed: {:?}", e),
}
```

## Integration with Other Crates

The `kgraph_utils` crate integrates with several other parts of the Hyperswitch ecosystem:

1. **euclid**: Uses the Euclid DSL for expressing constraints and values
2. **hyperswitch_constraint_graph**: Leverages the constraint graph structure for validation
3. **api_models**: Uses shared API models for payment method types and configurations
4. **common_enums**: Relies on common enumerations for payment types, currencies, etc.
5. **common_types**: Uses shared type definitions for payment-related structures

## Feature Flags

The crate supports several feature flags for customization:

- **dummy_connector**: Enables dummy connector support for testing
- **v1**: Compatibility with v1 API models
- **v2**: Compatibility with v2 API models

## Error Handling

The crate provides specialized error types:

- **KgraphError**: Custom error type for knowledge graph operations
- **Error Propagation**: Integration with the constraint graph error system
- **Error Mapping**: Conversion between API errors and graph errors

## Performance Considerations

- **Efficient Graph Construction**: Optimized algorithms for building constraint graphs
- **Memoization**: Support for caching validation results
- **Reusable Components**: Shared structures to minimize duplication

## Testing Strategy

The crate includes comprehensive tests:

- **Unit Tests**: Testing individual components and transformations
- **Integration Tests**: Testing graph construction and validation
- **Scenario Testing**: Testing specific payment scenarios
- **Edge Cases**: Testing boundary conditions and error cases

## Code Organization Patterns

- **Feature Gating**: Conditional compilation for different API versions
- **Type Conversion**: Systematic conversion between API and graph types
- **Builder Pattern**: Step-by-step construction of complex graph structures
- **Modular Design**: Separation of concerns between different aspects of graph management

## Conclusion

The `kgraph_utils` crate serves as a bridge between Hyperswitch's API models and its constraint validation system, enabling sophisticated payment routing rules based on merchant configurations. It translates business rules into a machine-readable format that can be efficiently validated at runtime, ensuring payments are routed correctly according to merchant-defined constraints.

## See Also

- [Euclid Overview](../euclid/overview.md)
- [Hyperswitch Constraint Graph Overview](../hyperswitch_constraint_graph/overview.md)
- [Euclid WASM Overview](../euclid_wasm/overview.md)
