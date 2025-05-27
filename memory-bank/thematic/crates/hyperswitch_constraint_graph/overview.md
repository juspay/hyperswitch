# Hyperswitch Constraint Graph Overview

The `hyperswitch_constraint_graph` crate provides a framework for modeling and validating domain-specific constraints using a graph-based approach. This document outlines its purpose, architecture, and usage within the Hyperswitch ecosystem.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `hyperswitch_constraint_graph` crate is responsible for:

1. Defining a graph-based framework for representing domain-specific constraints
2. Constructing constraint graphs with various node types (values, aggregators)
3. Validating contexts against defined constraint graphs
4. Providing detailed error traces for constraint violations
5. Offering visualization capabilities for constraint graphs (optional feature)

## Key Modules

The `hyperswitch_constraint_graph` crate is organized into the following key modules:

- **graph.rs**: Core implementation of the `ConstraintGraph` structure and validation logic
- **builder.rs**: Builder pattern implementation for constructing constraint graphs
- **types.rs**: Type definitions for nodes, edges, relations, and other graph components
- **error.rs**: Error types and analysis trace functionality for constraint violations
- **dense_map.rs**: Efficient data structure for storing graph elements

## Core Features

### Constraint Graph Model

The crate provides a flexible graph-based model for representing constraints:

- **Nodes**: Represent values or aggregation points in the constraint graph
- **Edges**: Connect nodes with specific strengths and relations
- **Domains**: Group constraints into logical domains for organization
- **Relations**: Define positive or negative relationships between nodes
- **Strengths**: Specify the importance of constraints (weak, normal, strong)

### Node Types

The framework supports different types of nodes for constraint modeling:

- **Value Nodes**: Represent specific values or keys to validate
- **All Aggregator**: Requires all connected constraints to be satisfied (AND logic)
- **Any Aggregator**: Requires at least one connected constraint to be satisfied (OR logic)
- **In Aggregator**: Validates that values are within a specified set

### Graph Construction

A builder pattern simplifies the creation of constraint graphs:

- **Value Nodes**: Create nodes for specific values or keys
- **Aggregator Nodes**: Create different types of aggregator nodes
- **Edges**: Connect nodes with specific relations and strengths
- **Domains**: Define logical groupings of constraints

### Constraint Validation

The core functionality validates whether a context satisfies the constraints:

- **Context Checking**: Validate a checking context against the constraint graph
- **Memoization**: Optimize validation by caching results
- **Cycle Detection**: Handle cycles in constraint graphs
- **Detailed Traces**: Generate detailed traces of validation failures

### Visualization

An optional feature (`viz`) provides graph visualization capabilities:

- **Graphviz Integration**: Generate DOT format representations of constraint graphs
- **Visual Debugging**: Visualize complex constraint relationships for debugging
- **Node and Edge Styling**: Visually distinguish different node types and edge strengths

## Public Interface

### Key Structs

```rust
pub struct ConstraintGraph<V: ValueNode> {
    pub domain: DenseMap<DomainId, DomainInfo>,
    pub domain_identifier_map: FxHashMap<DomainIdentifier, DomainId>,
    pub nodes: DenseMap<NodeId, Node<V>>,
    pub edges: DenseMap<EdgeId, Edge>,
    pub value_map: FxHashMap<NodeValue<V>, NodeId>,
    pub node_info: DenseMap<NodeId, Option<&'static str>>,
    pub node_metadata: DenseMap<NodeId, Option<Arc<dyn Metadata>>>,
}

pub struct ConstraintGraphBuilder<V: ValueNode = ()> {
    // Builder implementation details
}
```

### Key Traits

```rust
pub trait ValueNode: Clone + Debug + Eq + Hash + Serialize {
    type Key: Clone + Debug + Eq + Hash + Serialize;
    
    fn get_key(&self) -> Self::Key;
}

pub trait CheckingContext {
    type Value: ValueNode;
    
    fn check_presence(&self, node_value: &NodeValue<Self::Value>, strength: Strength) -> bool;
    fn get_values_by_key(&self, key: &<Self::Value as ValueNode>::Key) -> Option<Vec<Self::Value>>;
}
```

### Node Types

```rust
pub enum NodeType<V: ValueNode> {
    Value(NodeValue<V>),
    AllAggregator,
    AnyAggregator,
    InAggregator(FxHashSet<V>),
}

pub enum NodeValue<V: ValueNode> {
    Key(<V as ValueNode>::Key),
    Value(V),
}
```

## Usage Examples

### Creating a Constraint Graph

```rust
use hyperswitch_constraint_graph::{ConstraintGraphBuilder, Relation, Strength};

// Create a builder
let mut builder = ConstraintGraphBuilder::new();

// Create domains
let payment_domain = builder.make_domain("payment", "Payment constraints")?;
let user_domain = builder.make_domain("user", "User constraints")?;

// Create value nodes
let amount_node = builder.make_value_node(
    NodeValue::Key(AmountKey), 
    Some("payment_amount"), 
    None::<()>
);
let currency_node = builder.make_value_node(
    NodeValue::Value(Currency::USD), 
    Some("currency_usd"), 
    None::<()>
);

// Create aggregator node
let high_value_payment = builder.make_all_aggregator(
    &[amount_node, currency_node],
    Some("high_value_payment"),
    None::<()>,
    Some(payment_domain)
)?;

// Build the graph
let constraint_graph = builder.build();
```

### Validating Against a Context

```rust
use hyperswitch_constraint_graph::{ConstraintGraph, Relation, Strength, CheckingContext};

struct PaymentContext {
    amount: u64,
    currency: Currency,
    // other fields...
}

impl CheckingContext for PaymentContext {
    type Value = PaymentValue;
    
    fn check_presence(&self, node_value: &NodeValue<Self::Value>, strength: Strength) -> bool {
        match node_value {
            NodeValue::Key(AmountKey) => true,  // Key exists
            NodeValue::Value(PaymentValue::Amount(amount)) => self.amount >= *amount,
            NodeValue::Value(PaymentValue::Currency(currency)) => self.currency == *currency,
            // other checks...
        }
    }
    
    fn get_values_by_key(&self, key: &<Self::Value as ValueNode>::Key) -> Option<Vec<Self::Value>> {
        match key {
            AmountKey => Some(vec![PaymentValue::Amount(self.amount)]),
            CurrencyKey => Some(vec![PaymentValue::Currency(self.currency)]),
            // other keys...
        }
    }
}

// Validate context against constraint graph
let context = PaymentContext { amount: 1000, currency: Currency::USD };
let mut memo = FxHashMap::new();
let mut cycle_map = FxHashMap::new();

let result = constraint_graph.check_node(
    &context,
    high_value_payment,
    Relation::Positive,
    Strength::Normal,
    &mut memo,
    &mut cycle_map,
    None,
);
```

### Visualizing the Graph

```rust
#[cfg(feature = "viz")]
fn visualize_constraint_graph<V: ValueNode + NodeViz>(graph: &ConstraintGraph<V>) {
    // Generate DOT format representation
    let dot_string = graph.get_viz_digraph_string();
    
    // Write to file or use with Graphviz tools
    std::fs::write("constraint_graph.dot", dot_string).unwrap();
}
```

## Integration with Other Crates

The `hyperswitch_constraint_graph` crate integrates with several other parts of the Hyperswitch ecosystem:

1. **euclid**: The euclid crate uses constraint graphs to model and validate payment routing rules
2. **router**: The router crate leverages constraint graphs for routing decision validation
3. **common_enums**: Provides common enumerations used in constraint modeling

## Configuration Options

The crate offers the following feature flag:

- **viz**: Enables visualization capabilities using the `graphviz-rust` dependency

## Error Handling

The crate provides detailed error handling for constraint graph operations:

- **GraphError**: Top-level error type for graph operations
- **AnalysisTrace**: Detailed traces of constraint validation failures
- **Structured Errors**: Specific error types for different failure scenarios

## Performance Considerations

- **Memoization**: Results of node validation are cached to avoid redundant computation
- **Dense Maps**: Efficient storage of graph elements using dense maps
- **Cycle Detection**: Efficient handling of cycles in constraint graphs

## Thread Safety and Async Support

- The core data structures use interior mutability with thread-safe types like `Arc`
- Constraint validation is synchronous but can be used within async contexts
- No global mutable state is maintained

## Testing Strategy

The crate includes tests for:

- **Graph Construction**: Verify correct construction of constraint graphs
- **Constraint Validation**: Test validation of contexts against constraints
- **Error Handling**: Verify correct error generation and propagation
- **Edge Cases**: Test special cases like cycles and complex constraint relationships

## Conclusion

The `hyperswitch_constraint_graph` crate provides a powerful framework for modeling and validating domain-specific constraints in the Hyperswitch ecosystem. It is particularly valuable for payment routing rules and other scenarios requiring complex constraint validation, offering flexibility, performance, and detailed error reporting.

## See Also

- [Euclid Overview](../euclid/overview.md)
- [Router Configuration: Routing Strategies](../router/configuration/routing_strategies.md)
