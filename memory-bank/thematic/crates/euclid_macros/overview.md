# Euclid Macros Overview

The `euclid_macros` crate provides procedural macros that enhance the Euclid Domain-Specific Language (DSL) for payment routing rules within the Hyperswitch ecosystem. This document outlines its purpose, components, and usage.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `euclid_macros` crate is responsible for:

1. Providing procedural macros to enhance the Euclid DSL functionality
2. Simplifying common operations through code generation
3. Enabling enhanced type safety and expressiveness in routing rules
4. Supporting the knowledge representation capabilities of Euclid
5. Reducing boilerplate code for common patterns in the DSL

## Key Modules

The `euclid_macros` crate is organized into the following modules:

- **lib.rs**: Exports the procedural macros for external use
- **inner/enum_nums.rs**: Implementation of the `EnumNums` derive macro
- **inner/knowledge.rs**: Implementation of the `knowledge` procedural macro

## Core Features

### EnumNums Derive Macro

The `EnumNums` derive macro adds numeric conversion capabilities to enum types:

- **Automatic Indexing**: Generates a `to_num()` method that converts enum variants to their corresponding numeric indices
- **Type Safety**: Maintains type safety while allowing numeric operations
- **Zero Overhead**: Generates efficient code with no runtime performance cost
- **Variant Validation**: Ensures the macro is only applied to enums with unit variants

Example of the generated code:

```rust
impl SomeEnum {
    pub fn to_num(&self) -> usize {
        match self {
            Self::Variant1 => 0,
            Self::Variant2 => 1,
            Self::Variant3 => 2,
            // ...and so on for each variant
        }
    }
}
```

### Knowledge Procedural Macro

The `knowledge` procedural macro facilitates knowledge representation in the Euclid DSL:

- **Domain Knowledge Capture**: Enables representation of domain-specific knowledge
- **Rule Definition**: Simplifies the definition of complex routing rules
- **Compile-time Validation**: Validates rule definitions at compile-time
- **Error Reporting**: Provides clear error messages for invalid rule definitions

## Public Interface

### Exported Macros

```rust
// A derive macro that adds a to_num() method to enums with unit variants
#[proc_macro_derive(EnumNums)]
pub fn enum_nums(ts: TokenStream) -> TokenStream;

// A procedural macro for knowledge representation in the Euclid DSL
#[proc_macro]
pub fn knowledge(ts: TokenStream) -> TokenStream;
```

## Usage Examples

### Using EnumNums

```rust
use euclid_macros::EnumNums;

#[derive(EnumNums, Debug, Clone)]
enum PaymentMethod {
    Card,
    BankTransfer,
    Wallet,
    Cryptocurrency,
}

fn main() {
    let method = PaymentMethod::Wallet;
    
    // Convert variant to numeric index
    let method_index = method.to_num(); // Returns 2
    
    // Can be used in routing logic
    if method_index > PaymentMethod::Card.to_num() {
        println!("Using alternative payment method");
    }
}
```

### Using Knowledge Macro

```rust
use euclid::knowledge;

// Define routing knowledge
knowledge! {
    rule high_value_transaction {
        when payment.amount > 1000 && payment.currency == "USD" {
            route_to("processor_a", priority=1)
        } otherwise {
            route_to("processor_b", priority=2)
        }
    }
}
```

## Integration with Other Crates

The `euclid_macros` crate integrates with several other parts of the Hyperswitch ecosystem:

1. **euclid**: The main crate that implements the DSL for payment routing rules
2. **hyperswitch_constraint_graph**: Used in conjunction with knowledge representation for constraint validation
3. **router**: Utilizes the macros for efficient routing logic implementation

## Implementation Details

### EnumNums Implementation

The `EnumNums` derive macro:

1. Parses the input token stream to extract the enum definition
2. Validates that the enum only contains unit variants
3. Generates a match expression that maps each variant to its index
4. Implements the `to_num()` method on the enum type

The implementation uses the `syn` crate for parsing Rust syntax and the `quote` crate for code generation.

### Knowledge Macro Implementation

The `knowledge` procedural macro:

1. Parses a domain-specific syntax for defining routing rules
2. Validates the structure and semantics of the rules
3. Generates Rust code that implements the specified rules
4. Provides clear error messages for invalid rule definitions

## Error Handling

Both macros provide compile-time error messages when used incorrectly:

- **EnumNums**: Reports an error if applied to a non-enum type or an enum with non-unit variants
- **Knowledge**: Reports detailed errors for invalid rule syntax, semantic errors, or constraint violations

## Testing Strategy

The crate includes tests for:

- **Unit Tests**: Verify the correct implementation of each macro
- **Integration Tests**: Ensure the macros work correctly within the Euclid ecosystem
- **Error Cases**: Confirm appropriate error messages for invalid inputs

## Conclusion

The `euclid_macros` crate provides essential procedural macros that enhance the Euclid DSL for payment routing in Hyperswitch. These macros simplify the definition and implementation of routing rules, enable compile-time validation, and improve the expressiveness and type safety of the DSL.

## See Also

- [Euclid Overview](../euclid/overview.md)
- [Hyperswitch Constraint Graph](../hyperswitch_constraint_graph/overview.md)
