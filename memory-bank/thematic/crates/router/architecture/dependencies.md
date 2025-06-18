# Router Dependencies

---
**Parent:** [Router Overview](../overview.md)  
**Last Updated:** 2025-05-20  
**Related Files:**
- [Code Structure](./code_structure.md)
- [Entry Points](./entry_points.md)
---

[← Back to Router Overview](../overview.md)

## Overview

The router crate relies on several other crates in the Hyperswitch ecosystem as well as external dependencies. This document provides a detailed overview of these dependencies, their purposes, and the relationships between them.

## Core Hyperswitch Crates

The router crate integrates with several other Hyperswitch crates to provide a complete payment orchestration solution:

### API Models (`api_models`)

- **Purpose**: Defines the API request and response data structures
- **Usage in Router**: Used for parsing incoming requests and formatting outgoing responses
- **Relationship**: Router depends directly on api_models, but api_models does not depend on router

Key components used:
- Request models for payments, refunds, customers, etc.
- Response models for API operations
- Validation logic for API inputs
- Serialization/deserialization specifications

### Storage Implementation (`storage_impl`)

- **Purpose**: Provides a data access layer for persistent storage operations
- **Usage in Router**: Used for database operations (CRUD operations on payments, refunds, etc.)
- **Relationship**: Router depends on storage_impl for data access, some direct DB logic in router/src/db.rs is transitioning to storage_impl

Key components used:
- Repository implementations for various entities
- Query builders and executors
- Transaction management
- Connection pooling

### Hyperswitch Connectors (`hyperswitch_connectors`)

- **Purpose**: Implements communications with various payment processors
- **Usage in Router**: Used via the connector module to interact with payment processors
- **Relationship**: Router depends on hyperswitch_connectors but defines the high-level workflows

Key components used:
- Connector implementations for different payment processors
- Authentication logic for connector APIs
- Request/response transformers
- Error handling for connector-specific errors

### Euclid (`euclid`)

- **Purpose**: Provides a DSL and decision engine for routing and conditional logic
- **Usage in Router**: Used for defining and evaluating routing rules
- **Relationship**: Router depends on euclid for rule evaluation capabilities

Key components used:
- DSL for rule definition
- Rule evaluation engine
- Condition builders and evaluators
- Decision tree execution

### Redis Interface (`redis_interface`)

- **Purpose**: Abstracts interactions with Redis for caching and distributed operations
- **Usage in Router**: Used for caching, distributed locks, session state, etc.
- **Relationship**: Router depends on redis_interface for Redis operations

Key components used:
- Connection management
- Key-value operations
- Distributed locking mechanisms
- Pub/sub functionality

### Common Utils (`common_utils`)

- **Purpose**: Provides shared utility functions
- **Usage in Router**: Used throughout for common operations
- **Relationship**: Router depends on common_utils for utility functions

Key components used:
- Cryptography utilities
- Data conversion functions
- Date/time handling
- Common error handling patterns

### Common Enums (`common_enums`)

- **Purpose**: Defines shared enumeration types
- **Usage in Router**: Used for payment statuses, currencies, etc.
- **Relationship**: Router depends on common_enums for consistent type definitions

Key components used:
- Payment status enumerations
- Currency codes
- Error code enumerations
- Payment method types

### Diesel Models (`diesel_models`)

- **Purpose**: Defines database schema and models
- **Usage in Router**: Used via storage_impl for database operations
- **Relationship**: Router depends on diesel_models indirectly through storage_impl

Key components used:
- Database schema definitions
- ORM models
- Database migration specifications
- Query definitions

### Hyperswitch Domain Models (`hyperswitch_domain_models`)

- **Purpose**: Defines core business logic entities and types
- **Usage in Router**: Used for internal representation of domain concepts
- **Relationship**: Router depends on hyperswitch_domain_models for domain modeling

Key components used:
- Payment domain models
- Refund domain models
- Customer and payment method models
- Business rules and validation

### Router Environment (`router_env`)

- **Purpose**: Provides environment-specific configurations and logging setup
- **Usage in Router**: Used for configuration management and logging
- **Relationship**: Router depends on router_env for environment configuration

Key components used:
- Configuration loading and parsing
- Logging setup and management
- Environment detection
- Feature flag management

### Masking (`masking`)

- **Purpose**: Provides secure handling and logging of sensitive data
- **Usage in Router**: Used for masking sensitive data in logs and responses
- **Relationship**: Router depends on masking for PII protection

Key components used:
- Field masking implementations
- Secret detection algorithms
- Redaction strategies
- Secure logging patterns

## External Dependencies

Beyond the Hyperswitch ecosystem crates, router depends on several external libraries:

### Actix Web

- **Purpose**: Web framework for building the HTTP API
- **Usage in Router**: Primary framework for the API server
- **Version**: [Current version used in the project]

Key components used:
- HTTP server implementation
- Routing and middleware framework
- Request/response handling
- Websocket support (if applicable)

### Diesel

- **Purpose**: SQL ORM for Rust
- **Usage in Router**: Database operations (via storage_impl)
- **Version**: [Current version used in the project]

Key components used:
- Query building and execution
- Schema management
- Connection pooling
- Transaction handling

### Redis

- **Purpose**: Redis client library
- **Usage in Router**: Redis operations (via redis_interface)
- **Version**: [Current version used in the project]

Key components used:
- Redis command execution
- Connection management
- Pipelining and transactions
- PubSub operations

### Serde

- **Purpose**: Serialization/deserialization framework
- **Usage in Router**: JSON processing, configuration parsing
- **Version**: [Current version used in the project]

Key components used:
- JSON serialization/deserialization
- TOML configuration parsing
- Custom serialization logic
- Data validation

### Tokio

- **Purpose**: Asynchronous runtime
- **Usage in Router**: Async I/O, concurrency
- **Version**: [Current version used in the project]

Key components used:
- Async runtime
- Task scheduling
- I/O operations
- Synchronization primitives

### Futures

- **Purpose**: Future abstractions
- **Usage in Router**: Asynchronous programming patterns
- **Version**: [Current version used in the project]

Key components used:
- Future combinators
- Stream processing
- Async patterns
- Error handling

### Tracing

- **Purpose**: Distributed tracing and logging
- **Usage in Router**: Structured logging, trace context
- **Version**: [Current version used in the project]

Key components used:
- Span and event creation
- Context propagation
- Structured logging
- Integration with OpenTelemetry

### Metrics

- **Purpose**: Metrics collection and reporting
- **Usage in Router**: Performance and operational metrics
- **Version**: [Current version used in the project]

Key components used:
- Counter, gauge, histogram metrics
- Metric registry
- Prometheus integration
- Custom metric definitions

## Dependency Graph

The high-level dependency graph for the router crate:

```
router
  ├── api_models
  ├── storage_impl ────── diesel_models
  ├── hyperswitch_connectors
  ├── euclid
  ├── redis_interface
  ├── common_utils
  ├── common_enums
  ├── hyperswitch_domain_models
  ├── router_env
  ├── masking
  │
  ├── actix-web
  ├── diesel
  ├── redis
  ├── serde
  ├── tokio
  ├── futures
  ├── tracing
  └── metrics
```

## Dependency Management

The router crate manages its dependencies through several mechanisms:

### Cargo Features

Cargo features are used to conditionally include certain dependencies:

- API version features (e.g., `v1`, `v2`)
- Connector features (e.g., `stripe`, `adyen`)
- Storage options (e.g., `kv_store`)
- Optional capabilities (e.g., `frm`, `payouts`)

### Version Pinning

Dependencies are pinned to specific versions to ensure compatibility:

- Hyperswitch crates are typically pinned to exact versions
- External dependencies may use version ranges for minor updates

### Transitive Dependencies

Care is taken to manage transitive dependencies:

- Common dependencies are re-exported through common crates
- Version conflicts are resolved carefully
- Dependency duplication is minimized

## Dependency Update Process

When updating dependencies:

1. **Evaluation**: New versions are evaluated for compatibility and benefits
2. **Testing**: Comprehensive testing is performed to ensure no regressions
3. **Gradual Rollout**: Updates are rolled out gradually, starting with non-critical dependencies
4. **Monitoring**: System is monitored closely after dependency updates

## See Also

- [Code Structure Documentation](./code_structure.md)
- [Entry Points Documentation](./entry_points.md)
