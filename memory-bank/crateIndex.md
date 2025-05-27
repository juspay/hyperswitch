# Hyperswitch Crate Index

This document provides an overview of all the crates in the Hyperswitch project, their purposes, and relationships. Each crate has a specific responsibility in the overall architecture.

## Core Crates

### router
**Purpose**: The main application crate containing the core payment processing logic, API endpoints, and business logic.
**Key Components**:
- API routes and handlers
- Payment flow implementations
- Connector integrations
- Authentication and authorization
- Error handling
**Links**: [Detailed Documentation](./thematic/crates/router/overview.md)
**Dependencies**:
- api_models
- async-trait
- base64
- bytes
- chrono
- clap
- cookie
- diesel
- diesel_migrations
- dyn-clone
- error-stack
- futures
- hmac
- jsonwebtoken
- masking
- metrics
- num_enum
- opentelemetry
- pwhash
- redis
- reqwest
- router_derive
- router_env
- rust_decimal
- sea-orm
- serde
- serde_json
- sha2
- strum
- thiserror
- time
- tokio
- tracing
- utoipa
- validator
- vergen

### scheduler
**Purpose**: Handles scheduled tasks with two components: Producer (schedules tasks) and Consumer (executes tasks).
**Key Components**:
- Task scheduling
- Task execution
- Redis queue integration
**Links**: [Detailed Documentation](./thematic/crates/scheduler/overview.md) - Reviewed 2024-05-20
**Dependencies**:
- common_utils
- redis_interface
- serde
- tokio

### storage_impl
**Purpose**: Manages database interactions and data persistence.
**Key Components**:
- Repository implementations
- Database connection management
- Query builders and executors
**Links**: [Detailed Documentation](./thematic/crates/storage_impl/overview.md)
**Dependencies**:
- api_models
- common_enums
- common_utils
- diesel_models
- hyperswitch_domain_models
- masking
- redis_interface
- router_derive
- router_env
- async-bb8-diesel
- async-trait
- bb8
- bytes
- config
- crc32fast
- diesel
- dyn-clone
- error-stack
- futures
- moka
- once_cell
- serde
- serde_json
- thiserror
- tokio

### hyperswitch_connectors
**Purpose**: Implements integrations with various payment processors.
**Key Components**:
- Connector-specific API clients
- Request/response transformations
- Error handling and mapping
**Links**: [Detailed Documentation](./thematic/crates/hyperswitch_connectors/overview.md)
**Dependencies**:
- api_models
- common_enums
- common_types
- common_utils
- currency_conversion
- hyperswitch_domain_models
- masking
- redis_interface
- router_env
- serde
- serde_json
- tokio

## Model Crates

### api_models
**Purpose**: Defines the API request and response models.
**Key Components**:
- Request structures
- Response structures
- Validation logic
**Links**: [Detailed Documentation](./thematic/crates/api_models/overview.md)
**Dependencies**:
- common_enums
- common_types
- masking
- regex
- serde

### diesel_models
**Purpose**: Defines the database models using Diesel ORM.
**Key Components**:
- Table definitions
- Schema migrations
- Query methods
**Links**: [Detailed Documentation](./thematic/crates/diesel_models/overview.md)
**Dependencies**:
- common_enums
- common_types
- diesel
- num_enum
- serde

### hyperswitch_domain_models
**Purpose**: Defines the core domain models for the application.
**Key Components**:
- Business entities
- Domain logic
- Value objects
**Links**: [Detailed Documentation](./thematic/crates/hyperswitch_domain_models/overview.md)
**Dependencies**:
- common_enums
- common_types
- masking
- num_enum
- rust_decimal
- serde

### common_enums
**Purpose**: Defines shared enumerations used across the codebase.
**Key Components**:
- Status enums
- Type enums
- Error enums
**Links**: [Detailed Documentation](./thematic/crates/common_enums/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- serde
- strum

### common_types
**Purpose**: Defines shared type definitions used across the codebase.
**Key Components**:
- Payment-related types
- Domain entities
- Split payment structures
- Primitive wrapper types
- Customer and payment method types
**Links**: [Detailed Documentation](./thematic/crates/common_types/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- common_enums
- common_utils
- euclid
- diesel
- serde
- serde_json
- strum
- utoipa

## Utility Crates

### common_utils
**Purpose**: Provides utility functions and helpers used across the codebase.
**Key Components**:
- String manipulation
- Date/time utilities
- Cryptographic functions
- Logging utilities
**Links**: [Detailed Documentation](./thematic/crates/common_utils/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- common_enums
- masking
- router_env (optional)
- async-trait (optional)
- base64
- base64-serde
- blake3
- bytes
- diesel
- error-stack
- futures (optional)
- globset
- hex
- http
- md5
- nanoid
- nutype
- once_cell
- phonenumber
- quick-xml
- rand
- regex
- reqwest
- ring
- rust_decimal
- rustc-hash
- semver
- serde
- serde_json
- serde_urlencoded
- signal-hook (optional)
- strum
- thiserror
- time
- tokio (optional)
- url
- utoipa
- uuid
- openssl (optional)
- signal-hook-tokio (optional, target specific)

### router_env
**Purpose**: Manages environment configuration and setup.
**Key Components**:
- Environment variables
- Configuration loading
- Logging setup
**Links**: [Detailed Documentation](./thematic/crates/router_env/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- config
- serde
- tokio

### masking
**Purpose**: Handles masking of sensitive information.
**Key Components**:
- PII masking
- Card number masking
- Logging sanitization
**Links**: [Detailed Documentation](./thematic/crates/masking/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- serde

### router_derive
**Purpose**: Provides custom derive macros for the project.
**Key Components**:
- Procedural and attribute macros for code generation
- Database interaction utilities (DieselEnum, diesel_enum)
- API error handling (ApiError)
- Payment operation trait implementations
- Schema and validation macros
- Permission generation
**Links**: [Detailed Documentation](./thematic/crates/router_derive/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- proc-macro2
- quote
- syn
- indexmap
- serde_json
- strum

### config_importer
**Purpose**: Utility to convert a TOML configuration file to a list of environment variables.
**Key Components**:
- TOML parsing and transformation
- Environment variable generation
- Kubernetes integration
**Links**: [Detailed Documentation](./thematic/crates/config_importer/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- anyhow
- clap
- indexmap (optional)
- serde
- serde_json
- toml

### connector_configs
**Purpose**: Manages payment connector configurations and settings for Hyperswitch.
**Key Components**:
- Connector configuration structures
- Authentication types for different connector integrations
- Configuration loading based on environment
- Payment method configuration management
- Support for different connector types (payment, payout, authentication)
**Links**: [Detailed Documentation](./thematic/crates/connector_configs/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- api_models
- common_utils
- serde
- toml

### hsdev
**Purpose**: A simple diesel postgres migrator that uses TOML files.
**Key Components**:
- TOML configuration parsing
- Diesel migration integration
- Command-line interface
**Links**: [Detailed Documentation](./thematic/crates/hsdev/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- clap
- diesel
- diesel_migrations
- serde
- toml

## Feature-Specific Crates

### cards
**Purpose**: Handles card payment processing.
**Key Components**:
- Card validation
- Card tokenization
- Card-specific logic
- Secure handling of sensitive card data
- PCI DSS compliant implementations
**Links**: [Detailed Documentation](./thematic/crates/cards/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- common_utils
- masking
- router_env
- error-stack
- time
- regex
- serde

### payment_methods
**Purpose**: Implements various payment methods.
**Key Components**:
- Payment method validation
- Payment method-specific logic
**Links**: [Detailed Documentation](./thematic/crates/payment_methods/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- api_models
- common_enums
- common_types
- hyperswitch_domain_models
- masking
- serde

### pm_auth
**Purpose**: Handles payment method authentication.
**Key Components**:
- 3D Secure and other authentication protocols
- Authentication flows for bank accounts and payment methods
- Token exchange and credential retrieval
- Connector integrations (e.g., Plaid)
- Secure handling of payment method details
**Links**: [Detailed Documentation](./thematic/crates/pm_auth/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- api_models
- common_enums
- common_types
- common_utils
- hyperswitch_domain_models
- masking
- error-stack
- serde
- bytes
- thiserror

### currency_conversion
**Purpose**: Handles currency conversion operations.
**Key Components**:
- Exchange rate management
- Conversion logic
**Links**: [Detailed Documentation](./thematic/crates/currency_conversion/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- common_enums
- common_types
- redis_interface
- rust_decimal
- serde
- tokio

## Infrastructure Crates

### redis_interface
**Purpose**: Provides a Redis client and utilities.
**Key Components**:
- Connection management
- Caching operations
- Queue operations
**Links**: [Detailed Documentation](./thematic/crates/redis_interface/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- redis
- serde
- tokio

### drainer
**Purpose**: Service for processing queued tasks.
**Key Components**:
- Queue processing
- Task execution
**Links**: [Detailed Documentation](./thematic/crates/drainer/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- common_utils
- redis_interface
- serde
- tokio

### events
**Purpose**: Handles event processing and propagation.
**Key Components**:
- Event definitions
- Event handlers
- Event publishing
- Event context management
- Actix Web integration
**Links**: [Detailed Documentation](./thematic/crates/events/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- masking
- router_env
- error-stack
- serde
- serde_json
- thiserror
- time

## Integration Crates

### external_services
**Purpose**: Handles integration with external services.
**Key Components**:
- Email services (AWS SES, SMTP)
- File storage operations (AWS S3, local file system)
- Key management and encryption (AWS KMS, Hashicorp Vault)
- HTTP client operations and metrics
- gRPC client functionality
- CRM integrations (HubSpot)
- Secrets management
**Links**: [Detailed Documentation](./thematic/crates/external_services/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- api_models
- common_enums
- common_types
- common_utils
- hyperswitch_domain_models
- masking
- router_env
- error-stack
- reqwest
- serde
- tokio
- async-trait
- dyn-clone

### hyperswitch_interfaces
**Purpose**: Serves as a critical abstraction layer for payment connector integrations with standardized interfaces and traits.
**Key Components**:
- Connector Integration Interface (v1 and v2)
- ConnectorEnum and BoxedConnector abstractions
- Router data conversion for version compatibility
- Webhook handling interfaces
- Authentication and dispute handling interfaces
- Error handling mechanisms
**Links**: [Detailed Documentation](./thematic/crates/hyperswitch_interfaces/overview.md) - Reviewed 2025-05-20
**Dependencies**:
- api_models
- common_enums
- common_utils
- hyperswitch_domain_models
- masking
- router_derive
- router_env
- error-stack
- actix-web
- async-trait
- dyn-clone
- reqwest
- serde
- serde_json

## Documentation and Testing Crates

### openapi
**Purpose**: Generates OpenAPI specifications for the Hyperswitch API.
**Key Components**:
- API documentation and routes
- Schema generation
- Security scheme definitions
- API categorization by tags
**Links**: [Detailed Documentation](./thematic/crates/openapi/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- api_models
- common_enums
- common_types
- common_utils
- router_env
- serde_json
- utoipa

### test_utils
**Purpose**: Provides testing utilities for the Hyperswitch project, with a primary focus on running Postman collections using the Newman runner.
**Key Components**:
- Postman collection runner using Newman
- Connector authentication management
- Runtime collection modification
- CLI interface for test execution
- Module-specific test commands (connectors, users)
**Links**: [Detailed Documentation](./thematic/crates/test_utils/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- anyhow
- async-trait
- base64
- clap
- masking
- rand
- regex
- reqwest
- serde
- time
- tokio
- toml

## Analytics and Monitoring Crates

### analytics
**Purpose**: Handles analytics and reporting functionality.
**Key Components**:
- Data collection
- Reporting
- Metrics
**Links**: [Detailed Documentation](./thematic/crates/analytics/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- api_models
- common_enums
- common_types
- hyperswitch_domain_models
- serde
- sqlx

## Specialized Crates

### euclid
**Purpose**: Provides a Domain Specific Language (DSL) library for writing and evaluating dynamic payment routing rules.
**Key Components**:
- DSL for expressing payment routing rules
- Parsing and validation of rule expressions
- Rule compilation and optimization
- Rule execution and evaluation engines
- Integration with constraint graphs
**Links**: [Detailed Documentation](./thematic/crates/euclid/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- nom (optional)
- once_cell
- rustc-hash
- serde
- strum
- thiserror
- common_enums
- common_utils
- euclid_macros
- hyperswitch_constraint_graph

### kgraph_utils
**Purpose**: Provides utilities for constructing and working with knowledge graphs, with a primary focus on payment routing validation.
**Key Components**:
- Knowledge graph construction from merchant connector accounts
- Payment method type transformation
- Constraint configuration for routing rules
- Validation and testing for payment scenarios
**Links**: [Detailed Documentation](./thematic/crates/kgraph_utils/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- api_models
- common_enums
- common_types
- common_utils
- euclid
- hyperswitch_constraint_graph
- masking
- serde
- strum
- thiserror

### euclid_macros
**Purpose**: Provides procedural macros that enhance the Euclid Domain-Specific Language (DSL) for payment routing rules.
**Key Components**:
- EnumNums derive macro for numeric conversion of enum variants
- Knowledge procedural macro for domain-specific knowledge representation
- Code generation for routing rule expressions
- Compile-time validation of DSL syntax
**Links**: [Detailed Documentation](./thematic/crates/euclid_macros/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- proc-macro2
- quote
- rustc-hash
- strum
- syn

### euclid_wasm
**Purpose**: Provides WebAssembly (WASM) bindings for the Euclid Domain-Specific Language (DSL), enabling browser-based interactions with the payment routing rules engine.
**Key Components**:
- JavaScript binding layer for Euclid DSL functionality
- Routing rule validation and execution in web environments
- Currency conversion capabilities for frontend use
- Metadata access for payment routing components
- Connector configuration retrieval functions
**Links**: [Detailed Documentation](./thematic/crates/euclid_wasm/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- api_models
- common_enums
- common_types
- connector_configs
- currency_conversion
- euclid
- hyperswitch_constraint_graph
- kgraph_utils
- wasm-bindgen
- serde-wasm-bindgen

### hyperswitch_constraint_graph
**Purpose**: Framework for modeling and validating domain-specific constraints using a graph-based approach.
**Key Components**:
- Constraint graph model with nodes, edges, domains, and relations
- Node types including value nodes and various aggregators
- Graph construction with builder pattern
- Constraint validation and detailed error traces
- Visualization capabilities (optional feature)
**Links**: [Detailed Documentation](./thematic/crates/hyperswitch_constraint_graph/overview.md) - Reviewed 2025-05-27
**Dependencies**:
- erased-serde
- graphviz-rust (optional)
- rustc-hash
- serde
- strum
- thiserror

## Crate Dependency Graph

```mermaid
graph TD
    router --> storage_impl
    router --> hyperswitch_connectors
    router --> api_models
    router --> scheduler
    router --> common_utils
    router --> common_enums
    router --> common_types
    router --> hyperswitch_domain_models
    router --> redis_interface
    router --> payment_methods
    router --> cards
    
    scheduler --> redis_interface
    scheduler --> storage_impl
    
    hyperswitch_connectors --> hyperswitch_domain_models
    router --> common_utils
    
    storage_impl --> diesel_models
    storage_impl --> common_utils
    
    api_models --> common_enums
    api_models --> common_types
    
    diesel_models --> common_enums
    diesel_models --> common_types
    
    hyperswitch_domain_models --> common_enums
    hyperswitch_domain_models --> common_types
    
    payment_methods --> common_enums
    payment_methods --> common_types
    
    cards --> common_utils
```
