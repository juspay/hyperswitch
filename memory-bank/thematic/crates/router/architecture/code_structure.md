# Router Code Structure

---
**Parent:** [Router Overview](../overview.md)  
**Last Updated:** 2025-05-20  
**Related Files:**
- [Entry Points](./entry_points.md)
- [Dependencies](./dependencies.md)
---

[← Back to Router Overview](../overview.md)

## Overview

The `router` crate follows a structured organization pattern designed to separate concerns, promote maintainability, and support the complex payment orchestration functionality. This document outlines the physical file and directory structure of the crate.

## Top-Level Organization

The router crate is organized as follows:

```
router/
├── src/
│   ├── bin/                  # Binary entry points
│   ├── core/                 # Core business logic
│   ├── routes/               # API endpoint definitions
│   ├── connector/            # Interface to connectors
│   ├── services/             # Helper services
│   ├── db.rs                 # Database access
│   ├── middleware/           # Actix Web middleware
│   ├── types/                # Router-specific types
│   ├── utils/                # General utilities
│   ├── configs/              # Configuration loading
│   └── lib.rs                # Library entry point
└── Cargo.toml                # Crate manifest
```

## Key Directories and Files

### Binary Entry Points (`src/bin/`)

Contains main binary entry points:

- **`router.rs`**: Main API server application
- **`scheduler.rs`**: Scheduler service for background tasks

### Core Business Logic (`src/core/`)

The core directory contains the essential business logic:

```
core/
├── payments/               # Payment processing logic
│   ├── operations/         # Payment operations (create, confirm, etc.)
│   ├── flows/              # Payment process flows
│   └── transformers/       # Data transformation logic
├── refunds/                # Refund processing logic
├── payment_methods/        # Payment method handling
├── webhooks/               # Webhook processing
│   ├── inbound.rs          # Incoming webhook handling
│   └── outbound.rs         # Outgoing webhook delivery
├── routing/                # Payment routing logic
│   ├── strategies/         # Routing strategy implementations
│   └── rules/              # Rule definition and evaluation
├── mandates/               # Mandate management
├── authentication/         # Merchant authentication
└── errors/                 # Error definitions and handling
```

### API Routes (`src/routes/`)

Defines the HTTP endpoints:

```
routes/
├── payments.rs             # Payment endpoints
├── refunds.rs              # Refund endpoints
├── customers.rs            # Customer management endpoints
├── payment_methods.rs      # Payment method endpoints
├── mandates.rs             # Mandate management endpoints
├── webhooks.rs             # Webhook endpoints
├── health.rs               # Health check endpoints
├── admin.rs                # Administrative endpoints
├── metrics.rs              # Metrics endpoints
└── router.rs               # Route registration and configuration
```

### Connector (`src/connector/`)

Provides an interface to the `hyperswitch_connectors` crate:

```
connector/
├── connector.rs            # Connector interface definitions
├── utils.rs                # Connector utility functions
└── transformers/           # Connector-specific transformers
```

### Services (`src/services/`)

Contains helper services:

```
services/
├── api/                    # API-related services
├── authentication/         # Authentication services
├── authorization/          # Authorization services
├── db/                     # Database interaction services
├── redis/                  # Redis interaction services
├── logger/                 # Logging services
└── metrics/                # Metrics collection services
```

### Middleware (`src/middleware/`)

Contains Actix Web middleware components:

```
middleware/
├── auth.rs                 # Authentication middleware
├── logger.rs               # Logging middleware
├── cors.rs                 # CORS handling middleware
├── error_handler.rs        # Error handling middleware
└── metrics.rs              # Metrics collection middleware
```

### Types (`src/types/`)

Defines router-specific types:

```
types/
├── api.rs                  # API-related types
├── domain.rs               # Domain model types and extensions
├── storage.rs              # Storage-related types
├── transformers.rs         # Type transformation utilities
└── error.rs                # Error types
```

### Utilities (`src/utils/`)

General utility functions:

```
utils/
├── crypto.rs               # Cryptographic utilities
├── validation.rs           # Data validation utilities
├── transformers.rs         # Data transformation utilities
├── storage.rs              # Storage utilities
└── errors.rs               # Error handling utilities
```

### Configurations (`src/configs/`)

Configuration loading and management:

```
configs/
├── settings.rs             # Application settings
├── validator.rs            # Configuration validation
└── loader.rs               # Configuration loading
```

## Source File Organization

Within source files, the code typically follows this pattern:

1. **Imports**: External and internal module imports
2. **Constants**: Constants and static variables
3. **Types/Structs**: Type definitions specific to the module
4. **Trait Implementations**: Implementation of traits for defined types
5. **Functions**: Public and private functions
6. **Helper/Utility Functions**: Internal helper functions
7. **Tests**: Unit tests (typically in a submodule `mod tests`)

## Code Structure Principles

The router crate adheres to several structural principles:

### Separation of Concerns

- **Business Logic**: Isolated in the `core` module
- **API Interface**: Defined in the `routes` module
- **Data Access**: Managed through `services` and `db`
- **Cross-cutting Concerns**: Handled by `middleware`

### Layered Architecture

The code follows a layered pattern:

1. **API Layer**: Routes and controllers
2. **Business Logic Layer**: Core domain logic
3. **Data Access Layer**: Services and repository abstractions
4. **Infrastructure Layer**: Configuration, utilities, middleware

### Modularity

- Each module is designed to be self-contained
- Dependencies between modules are clearly defined
- Higher-level modules depend on lower-level modules
- Common functionality is abstracted into shared utilities

## Module Dependencies

The general dependency flow in the crate is:

```
routes → core → services → db/external services
     ↘       ↗
      middleware
```

This ensures a clean separation between API interfaces and business logic.

## See Also

- [Entry Points Documentation](./entry_points.md)
- [Dependencies Documentation](./dependencies.md)
