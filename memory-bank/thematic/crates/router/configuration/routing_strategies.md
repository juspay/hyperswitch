# Router Routing Strategies

---
**Parent:** [Router Overview](../overview.md)  
**Last Updated:** 2025-05-20  
**Related Files:**
- [Feature Flags](./feature_flags.md)
- [Core Module](../modules/core.md)
- [Payment Flows](../flows/payment_flows.md)
---

[← Back to Router Overview](../overview.md)

## Overview

Routing strategies are a core capability of the Hyperswitch payment orchestration platform, enabling intelligent distribution of payment transactions across multiple payment processors (connectors). This document details the routing strategy mechanisms implemented in the router crate, including rule configuration, evaluation, and execution.

## Routing Strategy Types

The router implements several types of routing strategies:

### Rule-based Routing

Rule-based routing uses predefined rules to select the appropriate payment processor based on transaction attributes:

- **Country-based Routing**: Routes transactions based on the customer's country
  - Example: Use Stripe for US customers, Adyen for European customers
  - Implementation: Uses country codes in routing rules

- **Currency-based Routing**: Routes transactions based on the payment currency
  - Example: Use different processors for USD, EUR, GBP transactions
  - Implementation: Uses currency codes in routing rules

- **Amount-based Routing**: Routes transactions based on the payment amount
  - Example: Use different processors for high-value vs. low-value transactions
  - Implementation: Uses amount thresholds in routing rules

- **Payment Method-based Routing**: Routes transactions based on the payment method
  - Example: Use specialized processors for specific payment methods
  - Implementation: Uses payment method types in routing rules

- **BIN-based Routing**: Routes card transactions based on the card BIN (Bank Identification Number)
  - Example: Route transactions based on card-issuing bank
  - Implementation: Uses BIN lookups and matching in routing rules

### Success Rate Optimization

Success rate optimization routes payments to processors with historically higher success rates:

- **General Success Rate**: Routes based on overall processor success rates
  - Implementation: Uses aggregated success rate metrics

- **Contextual Success Rate**: Routes based on success rates for similar transactions
  - Implementation: Uses success rates for specific countries, currencies, or payment methods

- **Learning-based Routing**: Adapts routing based on recent success patterns
  - Implementation: Uses time-weighted success metrics with higher weight for recent transactions

### Cost Optimization

Cost optimization routes payments to minimize processing fees:

- **Fee-based Routing**: Routes to processors with lowest transaction fees
  - Implementation: Uses configured fee structures for each processor

- **Volume-based Routing**: Routes to optimize volume tiers with processors
  - Implementation: Tracks volume with each processor and routes to optimize tier thresholds

- **Special Rate Routing**: Leverages special rate agreements for certain transaction types
  - Implementation: Applies special rate rules for qualifying transactions

### Fallback Handling

Fallback handling provides automatic recovery from failed transactions:

- **Sequential Fallback**: Tries alternative processors in a predefined sequence
  - Implementation: Maintains ordered list of fallback processors

- **Smart Fallback**: Selects fallback processors based on error type and success probability
  - Implementation: Maps error types to appropriate fallback strategies

- **Cascading Fallback**: Adjusts fallback strategy based on previous attempts
  - Implementation: Modifies fallback selection based on transaction history

### Volume Distribution

Volume distribution allocates transactions across multiple processors:

- **Percentage-based Distribution**: Distributes volume based on configured percentages
  - Implementation: Uses random selection weighted by configured percentages

- **Quota-based Distribution**: Distributes volume to meet minimum commitments
  - Implementation: Tracks volume and adjusts routing to meet quotas

- **Balanced Distribution**: Evenly distributes volume across processors
  - Implementation: Uses round-robin or least-recently-used algorithms

## Rule Definition and Evaluation

Routing rules are defined using the DSL (Domain Specific Language) provided by the `euclid` crate and evaluated using its decision engine:

### Rule Definition

Rules are typically defined in a structured format:

```json
{
  "name": "example_rule",
  "description": "Example routing rule",
  "connector_selection": {
    "priority_order": ["stripe", "adyen", "checkout"],
    "default_connector": "stripe"
  },
  "conditions": [
    {
      "field": "currency",
      "operator": "equals",
      "value": "USD"
    },
    {
      "field": "amount",
      "operator": "greater_than",
      "value": 1000
    },
    {
      "logical_operator": "and"
    }
  ]
}
```

### Condition Types

Rules can include various condition types:

- **Equality Conditions**: Matches exact values (e.g., currency = "USD")
- **Comparison Conditions**: Uses numerical comparisons (e.g., amount > 1000)
- **Inclusion Conditions**: Checks if a value is in a set (e.g., country in ["US", "CA"])
- **Pattern Matching**: Uses regex or pattern matching (e.g., card_bin matches "^4[0-9]{5}$")
- **Logical Operations**: Combines conditions with AND, OR, NOT operators

### Rule Evaluation

The router evaluates rules using these steps:

1. **Context Preparation**:
   - Extracts relevant data from the payment request
   - Prepares a context object with all fields needed for rule evaluation

2. **Rule Matching**:
   - Evaluates each rule's conditions against the context
   - Identifies all matching rules

3. **Rule Prioritization**:
   - Orders matching rules by priority
   - Selects the highest-priority matching rule

4. **Connector Selection**:
   - Uses the selected rule to determine the primary connector
   - Identifies fallback connectors if specified

## Rule Storage and Management

Routing rules can be managed through different mechanisms:

### Database Storage

- Rules are stored in the database as part of merchant configuration
- Merchants can define and update rules through the API
- Rules are versioned for audit and rollback capability

### Configuration Files

- Default rules can be defined in configuration files
- System-wide rules can be applied across all merchants
- Configuration-based rules are typically managed by platform operators

### Dynamic Updates

- Rules can be updated dynamically without system restart
- Rule changes take effect immediately for new transactions
- Rule performance metrics are collected to inform optimization

## Advanced Routing Features

The router implements several advanced routing capabilities:

### A/B Testing

- Routes a percentage of transactions to different processors
- Collects performance metrics for comparison
- Uses statistical analysis to identify optimal routing

### Time-based Routing

- Changes routing strategy based on time of day, day of week, etc.
- Accounts for processor maintenance windows
- Optimizes for regional processing patterns

### Routing Analytics

- Tracks routing decision effectiveness
- Provides insights into routing performance
- Suggests rule improvements based on analysis

### Custom Routing Functions

- Allows implementation of custom routing logic
- Supports complex use cases beyond standard rule capabilities
- Enables integration with external decision systems

## Routing Implementation Architecture

The routing implementation follows this architecture:

```
Routing Request → Rule Engine → Routing Decision → Connector Execution
                     ↑                  ↓
                     |                  |
                 Rule Store    Fallback Handling
```

### Core Components

- **Rule Engine**: Evaluates routing rules against transaction context
- **Rule Store**: Manages and retrieves applicable rules
- **Routing Decision**: Selects the appropriate connector based on rule evaluation
- **Fallback Handling**: Manages retry attempts with alternative connectors

## Configuration Example

A typical routing configuration might look like:

```json
{
  "routing_strategies": [
    {
      "name": "card_routing",
      "description": "Strategy for card payments",
      "payment_method_type": "card",
      "rules": [
        {
          "name": "us_high_value",
          "description": "US high-value transactions",
          "priority": 100,
          "connector_selection": {
            "primary": "stripe",
            "fallbacks": ["adyen", "checkout"]
          },
          "conditions": [
            {"field": "country", "operator": "equals", "value": "US"},
            {"field": "amount", "operator": "greater_than", "value": 10000},
            {"logical_operator": "and"}
          ]
        },
        {
          "name": "eu_transactions",
          "description": "European transactions",
          "priority": 90,
          "connector_selection": {
            "primary": "adyen",
            "fallbacks": ["stripe", "checkout"]
          },
          "conditions": [
            {"field": "region", "operator": "equals", "value": "Europe"}
          ]
        },
        {
          "name": "default_rule",
          "description": "Default routing",
          "priority": 0,
          "connector_selection": {
            "primary": "stripe",
            "fallbacks": ["adyen", "checkout"]
          },
          "conditions": []
        }
      ]
    }
  ]
}
```

## Performance Considerations

The routing system is designed with performance in mind:

- **Rule Caching**: Frequently used rules are cached for fast access
- **Efficient Evaluation**: Rules are evaluated using optimized algorithms
- **Parallel Evaluation**: Some rule sets can be evaluated in parallel
- **Minimal Database Access**: Reduces database queries during rule evaluation

## Dependencies

The routing implementation depends on several components:

- **euclid Crate**: Provides the DSL and decision engine for rule definition and evaluation
- **Core Module**: Implements the routing business logic
- **Storage Implementation**: Stores and retrieves routing rules
- **Metrics Collection**: Gathers data for success rate optimization

## See Also

- [Feature Flags Documentation](./feature_flags.md)
- [Core Module Documentation](../modules/core.md)
- [Payment Flows Documentation](../flows/payment_flows.md)
