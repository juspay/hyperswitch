# Router Crate Overview

The `router` crate is the central component of the Hyperswitch payment orchestration platform. It serves as the main application crate where all the core payment functionalities are implemented. This document provides a high-level overview with links to detailed documentation.

---
**Last Updated:** 2025-05-20  
**Documentation Status:** Expanded (Split into multiple files)
---

## Purpose

The router crate is responsible for:

1. Processing payment requests from clients
2. Routing payments to appropriate payment processors (connectors)
3. Managing end-to-end payment flows
4. Handling post-payment processes
5. Implementing sophisticated payment routing strategies
6. Providing robust APIs for all payment operations
7. Managing merchant authentication and authorization
8. Comprehensive error handling and formatting consistent responses

[→ Read more about the Router's Purpose](./purpose.md)

## Key Modules

The router crate is organized into several key modules:

- **Core**: Contains the core business logic for payment processing
- **Routes**: Defines the Actix Web API endpoints for the application
- **Connector**: Facilitates interaction with the `hyperswitch_connectors` crate
- **Services**: Provides various helper services used across the application
- **DB**: Handles database interactions (transitioning to `storage_impl`)
- **Middleware**: Contains Actix Web middleware components
- **Types**: Defines various data structures and types

[→ Detailed Core Module Documentation](./modules/core.md)  
[→ Detailed Routes Module Documentation](./modules/routes.md)  
[→ Detailed Services Module Documentation](./modules/services.md)  
[→ Detailed Middleware Module Documentation](./modules/middleware.md)

## Key Flows

The router implements several critical payment flows:

- **Payment Flow**: Creation, confirmation, authentication, capture, cancellation
- **Refund Flow**: Creation and status retrieval
- **Webhook Flow**: Processing incoming webhooks and sending outgoing notifications

[→ Detailed Payment Flows Documentation](./flows/payment_flows.md)  
[→ Detailed Refund Flows Documentation](./flows/refund_flows.md)  
[→ Detailed Webhook Flows Documentation](./flows/webhook_flows.md)

## Routing Logic

The router implements sophisticated, configurable payment routing logic:

- Rule-based routing
- Success rate optimization
- Cost optimization
- Fallback handling
- Volume distribution

[→ Detailed Routing Strategies Documentation](./configuration/routing_strategies.md)

## Error Handling

Comprehensive error handling includes:

- Connector-specific error normalization
- Validation error handling
- Internal error management
- Structured error responses

## Configuration

The router is highly configurable through:

- Environment variables
- Configuration files (TOML)
- Feature flags
- Merchant account configuration

[→ Detailed Feature Flags Documentation](./configuration/feature_flags.md)

## Dependencies

The router crate relies on several other key crates in the Hyperswitch ecosystem, including `api_models`, `storage_impl`, `hyperswitch_connectors`, and more.

[→ Detailed Dependencies Documentation](./architecture/dependencies.md)

## Code Structure

The router crate follows a structured organization pattern:

```
router/
├── src/
│   ├── bin/             # Binary entry points
│   ├── core/            # Core business logic
│   ├── routes/          # API endpoint definitions
│   ├── connector/       # Interface to connectors
│   ├── services/        # Helper services
│   ├── db.rs            # Database access
│   ├── middleware/      # Actix Web middleware
│   └── ...
└── Cargo.toml
```

[→ Detailed Code Structure Documentation](./architecture/code_structure.md)  
[→ Entry Points Documentation](./architecture/entry_points.md)

## Feature Flags

The router crate uses Cargo feature flags for API versions, connectors, processing modes, optional features, and storage options.

[→ Detailed Feature Flags Documentation](./configuration/feature_flags.md)

## Conclusion

The `router` crate is the heart of the Hyperswitch payment orchestration platform. Its modular design provides flexibility and extensibility, while its robust error handling and focus on security ensure reliable operation.

## See Also

- [System Patterns Documentation](/Users/arunraj/github/hyperswitch/memory-bank/systemPatterns.md)
- [Technical Context Documentation](/Users/arunraj/github/hyperswitch/memory-bank/techContext.md)
