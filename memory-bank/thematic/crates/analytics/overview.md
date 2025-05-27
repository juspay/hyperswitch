# Analytics Overview

The `analytics` crate provides comprehensive analytics, reporting, and search functionality for the Hyperswitch payment platform. This document provides an overview of its purpose, structure, and usage within the Hyperswitch ecosystem.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `analytics` crate is responsible for:

1. Collecting and processing data about payment transactions and related events
2. Providing metrics and dimensions for various analytics domains
3. Supporting analytics querying and reporting across multiple data domains
4. Facilitating data visualization and business intelligence
5. Integrating with search and analytics backends (OpenSearch)

## Key Modules

The `analytics` crate is organized into the following key domains and modules:

- **core**: Central functionality for analytics operations and domain info retrieval
- **payments**: Payment transaction analytics and metrics
- **payment_intents**: Analytics for payment intent lifecycle and status
- **refunds**: Refund transaction analytics and metrics
- **frm**: Fraud Risk Management analytics
- **disputes**: Dispute handling and chargeback analytics
- **outgoing_webhook_event**: Analytics for outgoing webhook events
- **sdk_events**: SDK-generated event analytics
- **auth_events**: Authentication event analytics
- **api_event**: API usage and performance analytics
- **active_payments**: Analytics for currently active payment operations
- **metrics**: Shared metrics functionality and utilities

Each domain has its own metrics implementation, often with sessionized metrics for analyzing user sessions.

## Core Features

### Multi-Domain Analytics

The analytics crate supports multiple analytics domains, each focusing on a specific aspect of the payment system:

- Payment processing metrics and dimensions
- Payment intent lifecycle analytics
- Refund analytics
- Fraud risk management
- Authentication events
- API usage
- Dispute handling
- SDK events

### Metrics and Dimensions

For each analytics domain, the crate provides:

- **Metrics**: Quantitative measurements like counts, amounts, and rates
- **Dimensions**: Qualitative attributes for grouping and filtering data
- **Aggregation**: Methods for combining and summarizing metrics
- **Time-based analysis**: Support for time-series analysis and trends

### Integration with Data Stores

The crate integrates with multiple data storage systems:

- **OpenSearch**: For search and analytics capabilities
- **PostgreSQL/SQLx**: For relational data storage and querying
- **AWS Lambda**: For serverless analytics processing

### Reporting Functionality

The analytics crate facilitates generating reports and insights:

- Performance dashboards
- Transaction volume analysis
- Success/failure rate tracking
- Trend identification
- Anomaly detection

## Public Interface

The analytics crate exposes interfaces for querying and retrieving analytics data.

### Key Domains

```rust
pub enum AnalyticsDomain {
    Payments,
    PaymentIntents,
    Refunds,
    Frm,
    SdkEvents,
    AuthEvents,
    ApiEvents,
    Dispute,
}
```

### Main Functions

```rust
pub async fn get_domain_info(
    domain: AnalyticsDomain,
) -> crate::errors::AnalyticsResult<GetInfoResponse> {
    // Implementation returns metrics and dimensions for the requested domain
}
```

## Usage Examples

### Retrieving Metrics for a Domain

```rust
use analytics::{types::AnalyticsDomain, core::get_domain_info};

async fn get_payment_analytics() -> Result<(), Error> {
    let analytics_info = get_domain_info(AnalyticsDomain::Payments).await?;
    
    // Use metrics and dimensions from analytics_info
    let available_metrics = analytics_info.metrics;
    let available_dimensions = analytics_info.dimensions;
    
    // Process analytics data...
    
    Ok(())
}
```

## Integration with Other Crates

The `analytics` crate integrates with several other parts of the Hyperswitch ecosystem:

1. **api_models**: Uses shared API models for analytics responses and requests
2. **common_utils**: Leverages common utilities for processing and formatting
3. **hyperswitch_domain_models**: Accesses domain models for analytics context
4. **storage_impl**: Interacts with the storage layer for data retrieval
5. **router_env**: Uses environment configuration and logging capabilities

## Configuration Options

The crate supports feature flags for version compatibility:

- **v1**: Enables compatibility with v1 API and data models
- **v2**: Enables compatibility with v2 API and data models

## Error Handling

The analytics crate provides its own error type for handling analytics-specific errors:

```rust
// Example error handling (structure inferred from usage patterns)
pub type AnalyticsResult<T> = std::result::Result<T, AnalyticsError>;

// Error propagation follows the error-stack pattern used throughout Hyperswitch
```

## Performance Considerations

- **Efficient Data Retrieval**: The crate implements optimized queries for analytics data retrieval
- **Caching**: Where appropriate, results are cached to reduce database load
- **Asynchronous Processing**: All operations are asynchronous to prevent blocking
- **Query Optimization**: Complex analytics queries are optimized for performance

## Thread Safety and Async Support

The analytics crate is designed for asynchronous operation, with all public interfaces using the `async/await` pattern. It is thread-safe and can be used concurrently.

## Testing Strategy

The crate is tested through:

- Unit tests for individual components
- Integration tests for end-to-end analytics flows
- Performance tests for query optimization

## Conclusion

The `analytics` crate serves as the central analytics engine for the Hyperswitch platform, providing critical insights into payment operations, user behavior, and system performance across multiple domains.

## See Also

- [API Models Documentation](../api_models/overview.md)
- [Storage Implementation Documentation](../storage_impl/overview.md)
- [Router Documentation](../router/overview.md)
