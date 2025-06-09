# Euclid Overview

The `euclid` crate provides a Domain-Specific Language (DSL) for defining and executing payment routing rules within the Hyperswitch ecosystem. This document outlines its purpose, architecture, and usage.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `euclid` crate is responsible for:

1. Defining a language for expressing payment routing rules
2. Parsing and validating routing rule expressions
3. Compiling rules into executable representations
4. Evaluating rules against payment context to determine routing decisions
5. Providing flexible routing strategies based on various criteria

## Key Modules

The `euclid` crate is organized into the following key modules:

- **frontend**: Handles parsing and initial processing of routing rules
  - **ast**: Abstract Syntax Tree representation of routing rules
  - **dir**: Directory-related functionality for organizing rules
  - **vir**: Value Intermediate Representation for processed rules
  
- **backend**: Handles rule execution and evaluation
  - **inputs**: Defines input data structures for rule evaluation
  - **interpreter**: Core rule interpreter for executing routing decisions
  - **vir_interpreter**: Optimized interpreter for Value Intermediate Representation (enabled via feature flag)

- **dssa**: Data Structure Specific Analysis tools for routing rules

- **enums**: Common enumerations used across the crate

- **types**: Core type definitions for the DSL

## Core Features

### DSL for Routing Rules

Euclid provides a specialized language for expressing payment routing logic:

- **Condition-based Routing**: Define rules based on payment attributes
- **Priority-based Selection**: Set priority levels for routing decisions
- **Fallback Mechanisms**: Define fallback strategies when primary routes fail
- **Connector-specific Rules**: Create rules tailored to specific payment processors
- **Payment Method Routing**: Route based on payment method characteristics

### Rule Parsing and Compilation

The frontend components handle rule definition and processing:

- **Parsing**: Transform textual rule definitions into structured representations
- **Validation**: Ensure rules are well-formed and semantically valid
- **Optimization**: Apply optimizations to improve rule execution performance
- **Transformation**: Convert between different rule representations

### Rule Execution

The backend components handle rule evaluation and execution:

- **Contextual Evaluation**: Evaluate rules against payment context
- **Decision Making**: Determine routing based on rule evaluation
- **Result Production**: Generate actionable routing decisions
- **Error Handling**: Handle failures during rule evaluation

### Integration with Constraint Graph

Euclid integrates with the `hyperswitch_constraint_graph` crate:

- **Constraint Representation**: Express routing constraints in a graph-based format
- **Constraint Checking**: Validate routing decisions against constraints
- **Visualization**: Support for visualizing routing constraints

## Public Interface

### Key Traits

```rust
pub trait EuclidBackend<O>: Sized {
    type Error: serde::Serialize;

    fn with_program(program: ast::Program<O>) -> Result<Self, Self::Error>;

    fn execute(&self, input: BackendInput) -> Result<BackendOutput<O>, Self::Error>;
}
```

### Important Structs

```rust
pub struct BackendOutput<O> {
    pub rule_name: Option<String>,
    pub connector_selection: O,
}

pub struct BackendInput {
    // Input data for rule evaluation
    // (Simplified for documentation)
}
```

## Usage Examples

### Defining a Routing Rule

```rust
// Example of a routing rule in the Euclid DSL syntax
// Note: Actual syntax may vary and is not fully represented here
rule high_value_transactions {
    when payment.amount > 1000 && payment.currency == "USD" {
        route_to("processor_a", priority=1)
    } otherwise {
        route_to("processor_b", priority=2)
    }
}
```

### Executing a Rule

```rust
use euclid::backend::{BackendInput, EuclidBackend, InterpreterBackend};
use euclid::frontend::ast;

fn execute_routing_rules(
    program: ast::Program<String>,
    payment_data: PaymentData,
) -> Result<String, Error> {
    // Create backend interpreter with the program
    let backend = InterpreterBackend::with_program(program)?;
    
    // Create input from payment data
    let input = BackendInput::from_payment_data(payment_data);
    
    // Execute the program
    let output = backend.execute(input)?;
    
    // Return the selected connector
    Ok(output.connector_selection)
}
```

## Integration with Other Crates

The `euclid` crate integrates with several other parts of the Hyperswitch ecosystem:

1. **router**: The router crate uses euclid to make routing decisions based on configured rules
2. **hyperswitch_constraint_graph**: Provides constraint validation for routing decisions
3. **euclid_macros**: Procedural macros that enhance the euclid DSL functionality
4. **common_enums**: Uses shared enumerations for consistent types across the system

## Configuration Options

The crate offers several feature flags to control its behavior:

- **ast_parser**: Enables the AST parser functionality (depends on `nom`)
- **valued_jit**: Enables the optimized VIR interpreter backend
- **dummy_connector**: Enables support for test/dummy connectors
- **payouts**: Enables support for payout-specific routing
- **v2**: Compatibility with v2 API models

## Error Handling

The crate uses a rich error handling approach:

- Backend-specific error types that implement serde::Serialize
- Detailed error information for debugging rule execution issues
- Clear error messages for rule validation failures

## Performance Considerations

- **Compilation vs. Interpretation**: Rules can be compiled for better performance or interpreted for flexibility
- **Optimization Passes**: The frontend applies optimizations to improve rule execution performance
- **Benchmarking**: The crate includes benchmarks to measure and improve performance
- **Caching**: Internal caching mechanisms improve repeated rule execution

## Thread Safety and Async Support

- The core data structures are designed to be thread-safe
- Backends are designed to be stateless where possible for concurrent execution
- Rule evaluation is synchronous, but the results can be used in async contexts

## Testing Strategy

- **Unit Tests**: Each component has focused unit tests
- **Integration Tests**: End-to-end tests verify correct rule evaluation
- **Benchmarks**: Performance benchmarking via Criterion framework
- **Property Testing**: Tests for rule correctness across a range of inputs

## Conclusion

The `euclid` crate provides a powerful, flexible DSL for defining payment routing rules in the Hyperswitch ecosystem. It allows for expressive, maintainable routing strategies that can evolve with business requirements while providing efficient evaluation at runtime.

## See Also

- [Router Overview](../router/overview.md)
- [Hyperswitch Constraint Graph](../hyperswitch_constraint_graph/overview.md)
- [Euclid Macros](../euclid_macros/overview.md)
- [Router Configuration: Routing Strategies](../router/configuration/routing_strategies.md)
