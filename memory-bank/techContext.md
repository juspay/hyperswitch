# Hyperswitch Technical Context

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Technology Stack

### Programming Language
-   **Rust** (stable channel, e.g., 1.80.0 at time of writing; refer to `Cargo.toml` for the exact version used by the current build) - The entire backend is written in Rust, leveraging its performance, safety, and concurrency features.

### Core Libraries and Frameworks
Key libraries include (refer to `Cargo.toml` for specific versions):
-   **Actix Web** (e.g., v4.x) - Web framework for building the API server.
-   **Diesel** (e.g., v2.x) - ORM for database interactions with PostgreSQL.
-   **Tokio** (e.g., v1.x) - Asynchronous runtime for handling concurrent operations.
-   **Serde** (e.g., v1.x) - Serialization/deserialization framework.
-   **Reqwest** (e.g., v0.11.x) - HTTP client for making API calls to payment processors.
-   **Clap** (e.g., v4.x) - Command-line argument parsing.
-   **Error-stack** (e.g., v0.4.x) - Error handling library.
-   **Tracing** (e.g., v0.1.x) - Logging and instrumentation framework.

### Database and Storage
-   **PostgreSQL** - Primary database for storing payment data, merchant information, etc.
-   **Diesel ORM** - Used for type-safe SQL queries and database interactions.
-   **Redis** - Used for caching and as a queue for the scheduler. Hyperswitch supports Clustered Redis, which is the preferred setup for production.

### Payment Processing
-   Multiple payment connector integrations (Stripe, Square, etc.).
-   Support for various payment methods (cards, wallets, bank transfers, BNPL).
-   PCI DSS compliant payment data handling.

### Monitoring and Observability
-   **OpenTelemetry** - For metrics and traces collection.
-   **Prometheus** - For metrics storage and querying.
-   **Loki** - For log aggregation.
-   **Tempo** - For distributed tracing.
-   **Grafana** - For visualization of metrics, logs, and traces.

### Deployment and Infrastructure
-   **Docker** - Containerization of services.
-   **Docker Compose** - Local development and deployment orchestration.
-   **Kubernetes**: Primary and officially supported production deployment target. Helm charts (e.g., `hyperswitch-helm`) are available for streamlined deployment.

## Project Structure

### Workspace Organization
The project is organized as a Rust workspace with multiple crates, each with specific responsibilities:

```
crates/
├── analytics/            # Analytics and reporting functionality
├── api_models/           # API request/response models
├── cards/                # Card payment processing
├── common_enums/         # Shared enumerations
├── common_types/         # Shared type definitions
├── common_utils/         # Utility functions and helpers
├── config_importer/      # Configuration loading and management
├── connector_configs/    # Payment connector configurations
├── currency_conversion/  # Currency conversion utilities
├── diesel_models/        # Database models using Diesel ORM
├── drainer/              # Service for processing queued tasks
├── euclid/               # Provides a Domain Specific Language (DSL) library for writing and evaluating dynamic payment routing rules.
├── events/               # Event handling and processing
├── external_services/    # Integration with external services
├── hyperswitch_connectors/ # Payment connector implementations
├── hyperswitch_domain_models/ # Core domain models
├── hyperswitch_interfaces/ # Interface definitions
├── masking/              # Data masking for sensitive information
├── openapi/              # OpenAPI specification generation
├── payment_methods/      # Payment method implementations
├── pm_auth/              # Payment method authentication
├── redis_interface/      # Redis client and utilities
├── router/               # Main application logic
├── router_derive/        # Custom derive macros
├── router_env/           # Environment configuration
├── scheduler/            # Task scheduling service
├── storage_impl/         # Storage implementation
└── test_utils/           # Testing utilities
```

### Key Crates

#### Router
The main crate containing the core payment processing logic, API endpoints, and business logic. It includes:
- API routes and handlers
- Payment flow implementations
- Connector integrations (utilizing `hyperswitch_connectors`)
- Routing rule evaluation (utilizing `euclid`)
- Authentication and authorization
- Error handling

#### Scheduler
Handles scheduled tasks with two components:
- Producer: Schedules tasks and adds them to the Redis queue.
- Consumer: Executes tasks from the Redis queue.

#### Storage_impl
Manages database interactions and data persistence using `diesel_models`:
- Repository implementations
- Database connection management
- Query builders and executors

#### Hyperswitch_connectors
Implements integrations with various payment processors:
- Connector-specific API clients
- Request/response transformations
- Error handling and mapping

## Development Environment

### Local Development Setup
The project can be run locally using Docker Compose with the following services:
- PostgreSQL database
- Redis (configurable for standalone or clustered simulation) for caching and queuing
- Hyperswitch server (router)
- Scheduler (producer and consumer)
- Web client for frontend (if applicable to the test setup)
- Control center for administration (if applicable to the test setup)
- Monitoring stack (optionally included in local Docker Compose setup)

### Configuration
- Configuration files in TOML format (e.g., `development.toml`, `config.example.toml`).
- Environment-specific configurations.
- Feature flags for enabling/disabling functionality.

### Testing
- Unit tests throughout the codebase (`cargo test`).
- Integration tests for API endpoints and flows.
- Test utilities (`test_utils` crate) for mocking dependencies and setting up test data.

## Deployment Options

### Docker Compose
For local development, testing, and simpler single-node deployments. Profiles allow for different setups:
- Basic setup: Core services only.
- Full setup: Including monitoring, scheduler, etc.
- Clustered Redis: Docker Compose profiles can simulate production-like environments with Clustered Redis.

### Kubernetes
The recommended method for production deployments. Official Helm charts (e.g., `hyperswitch-helm`) facilitate:
- Horizontal scaling of components (Router, Scheduler).
- High availability and fault tolerance.
- Load balancing.
- Simplified configuration management.
- Integration with production-grade monitoring and observability.
- Support for Clustered Redis is preferred for production.

## Feature Flags and Versioning

The project uses feature flags extensively in `Cargo.toml` to control functionality:
- Version-specific features (e.g., `v1`, `v2` API versions or behaviors).
- Optional components (e.g., email sending, specific FRM integrations).
- Connector-specific features or compilation.
- Performance optimizations or alternative implementations.

## Security Considerations

- PCI DSS compliance for payment data.
- GDPR compliance for personal data.
- Encryption of sensitive information at rest and in transit (leveraging the Locker component).
- Secure API authentication (e.g., API keys, JWTs).
- Input validation and sanitization at API boundaries.

## Links to Detailed Documentation

- [Router Crate](./thematic/crates/router/overview.md)
- [Scheduler Crate](./thematic/crates/scheduler/overview.md)
- [Connectors Crate](./thematic/crates/hyperswitch_connectors/overview.md)
- [Database Models](./thematic/crates/diesel_models/overview.md)
- [API Models](./thematic/crates/api_models/overview.md)
- [Development Setup in docs](../docs/try_local_system.md)
- [Deployment on AWS in docs](../docs/one_click_setup.md)

## Document History

| Date | Changes |
|------|---------|
| 2025-05-27 | Updated documentation links to point to existing files, added metadata |
| Prior | Initial version |
