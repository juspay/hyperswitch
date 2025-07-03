---
title: Crate Functionality Index
last_updated: 2025-05-27
position: 1
---

# Crate Functionality Index

This index categorizes all crates in the Hyperswitch project by their functionality, providing quick access to relevant documentation.

## Core Crates

These crates form the essential functionality of Hyperswitch:

- [Router](../thematic/crates/router/overview.md) - Main routing and processing logic, API endpoints, business logic
- [Hyperswitch Connectors](../thematic/crates/hyperswitch_connectors/overview.md) - Implements integrations with various payment processors
- [Storage Implementation](../thematic/crates/storage_impl/overview.md) - Manages database interactions and data persistence
- [Scheduler](../thematic/crates/scheduler/overview.md) - Handles scheduled tasks with Producer and Consumer components

## Model Crates

These crates define the data models and structures:

- [API Models](../thematic/crates/api_models/overview.md) - Defines API request and response models
- [Diesel Models](../thematic/crates/diesel_models/overview.md) - Defines database models using Diesel ORM
- [Hyperswitch Domain Models](../thematic/crates/hyperswitch_domain_models/overview.md) - Defines core domain models
- [Common Enums](../thematic/crates/common_enums/overview.md) - Defines shared enumerations
- [Common Types](../thematic/crates/common_types/overview.md) - Defines shared type definitions

## Utility Crates

These crates provide supporting functionality:

- [Common Utils](../thematic/crates/common_utils/overview.md) - Provides utility functions and helpers
- [Router Env](../thematic/crates/router_env/overview.md) - Manages environment configuration and setup
- [Masking](../thematic/crates/masking/overview.md) - Handles masking of sensitive information
- [Router Derive](../thematic/crates/router_derive/overview.md) - Provides custom derive macros
- [Config Importer](../thematic/crates/config_importer/overview.md) - Utility to convert TOML configuration to environment variables
- [Connector Configs](../thematic/crates/connector_configs/overview.md) - Manages payment connector configurations
- [HSdev](../thematic/crates/hsdev/overview.md) - A simple diesel postgres migrator

## Feature-Specific Crates

These crates implement specific features:

- [Cards](../thematic/crates/cards/overview.md) - Handles card payment processing
- [Payment Methods](../thematic/crates/payment_methods/overview.md) - Implements various payment methods
- [PM Auth](../thematic/crates/pm_auth/overview.md) - Handles payment method authentication
- [Currency Conversion](../thematic/crates/currency_conversion/overview.md) - Handles currency conversion operations

## Infrastructure Crates

These crates manage infrastructure concerns:

- [Redis Interface](../thematic/crates/redis_interface/overview.md) - Provides Redis client and utilities
- [Drainer](../thematic/crates/drainer/overview.md) - Service for processing queued tasks
- [Events](../thematic/crates/events/overview.md) - Handles event processing and propagation

## Integration Crates

These crates handle integration with external systems:

- [External Services](../thematic/crates/external_services/overview.md) - Handles integration with external services
- [Hyperswitch Interfaces](../thematic/crates/hyperswitch_interfaces/overview.md) - Abstraction layer for payment connector integrations

## Documentation and Testing Crates

These crates support documentation and testing:

- [OpenAPI](../thematic/crates/openapi/overview.md) - Generates OpenAPI specifications
- [Test Utils](../thematic/crates/test_utils/overview.md) - Provides testing utilities

## Analytics and Monitoring Crates

These crates handle analytics and monitoring:

- [Analytics](../thematic/crates/analytics/overview.md) - Handles analytics and reporting functionality

## Specialized Crates

These crates provide specialized functionality:

- [Euclid](../thematic/crates/euclid/overview.md) - DSL library for payment routing rules
- [Euclid Macros](../thematic/crates/euclid_macros/overview.md) - Procedural macros for Euclid DSL
- [Euclid WASM](../thematic/crates/euclid_wasm/overview.md) - WASM bindings for Euclid DSL
- [Hyperswitch Constraint Graph](../thematic/crates/hyperswitch_constraint_graph/overview.md) - Framework for constraint modeling
- [KGraph Utils](../thematic/crates/kgraph_utils/overview.md) - Utilities for knowledge graphs

## Crate Dependency Relationships

The Hyperswitch crates have various dependencies between them. The core dependency relationships are:

- Router depends on most other crates, particularly Storage Impl, Hyperswitch Connectors, and API Models
- Model crates (API Models, Diesel Models, Domain Models) depend on Common Enums and Common Types
- Storage Impl depends on Diesel Models and Common Utils
- Hyperswitch Connectors depend on Domain Models and Common Utils
- Scheduler depends on Redis Interface and Storage Impl

For a complete dependency graph, see the [System Patterns](../systemPatterns.md) document.

## Crate Functionality Matrix

| Category | Storage | API | Processing | Configuration | Utilities |
|----------|---------|-----|------------|---------------|-----------|
| Router | ✅ | ✅ | ✅ | ✅ | ✅ |
| Storage Impl | ✅ | ❌ | ❌ | ✅ | ✅ |
| API Models | ❌ | ✅ | ❌ | ❌ | ❌ |
| Scheduler | ✅ | ❌ | ✅ | ✅ | ❌ |
| Hyperswitch Connectors | ❌ | ❌ | ✅ | ✅ | ❌ |
| Common Utils | ❌ | ❌ | ❌ | ❌ | ✅ |
| Redis Interface | ✅ | ❌ | ❌ | ✅ | ✅ |
| Payment Methods | ❌ | ❌ | ✅ | ✅ | ❌ |
| Cards | ❌ | ❌ | ✅ | ❌ | ❌ |

## Search by Functionality

### Authentication & Authorization
- [Router](../thematic/crates/router/overview.md) - Authentication middleware and logic
- [PM Auth](../thematic/crates/pm_auth/overview.md) - Payment method authentication

### Data Storage & Persistence
- [Storage Implementation](../thematic/crates/storage_impl/overview.md) - Database interactions
- [Redis Interface](../thematic/crates/redis_interface/overview.md) - Redis storage and caching
- [Diesel Models](../thematic/crates/diesel_models/overview.md) - Database models

### Payment Processing
- [Router](../thematic/crates/router/overview.md) - Core payment flows
- [Payment Methods](../thematic/crates/payment_methods/overview.md) - Payment method implementations
- [Cards](../thematic/crates/cards/overview.md) - Card payment processing
- [Currency Conversion](../thematic/crates/currency_conversion/overview.md) - Currency operations

### Integration
- [Hyperswitch Connectors](../thematic/crates/hyperswitch_connectors/overview.md) - Payment processor integrations
- [External Services](../thematic/crates/external_services/overview.md) - External service integrations
- [Hyperswitch Interfaces](../thematic/crates/hyperswitch_interfaces/overview.md) - Connector interfaces

### Configuration & Environment
- [Router Env](../thematic/crates/router_env/overview.md) - Environment configuration
- [Config Importer](../thematic/crates/config_importer/overview.md) - Configuration utilities
- [Connector Configs](../thematic/crates/connector_configs/overview.md) - Connector configuration

### Routing & Rules
- [Euclid](../thematic/crates/euclid/overview.md) - Routing rules DSL
- [Hyperswitch Constraint Graph](../thematic/crates/hyperswitch_constraint_graph/overview.md) - Constraint modeling
- [KGraph Utils](../thematic/crates/kgraph_utils/overview.md) - Knowledge graph utilities

## Related Resources

- [System Patterns](../systemPatterns.md) - System architecture patterns
- [Tech Context](../techContext.md) - Technology context and stack information
- [Router Documentation](../thematic/crates/router/overview.md) - Documentation for the core Router crate
