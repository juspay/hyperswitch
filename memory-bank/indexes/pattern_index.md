---
title: Pattern Index
last_updated: 2025-05-27
position: 1
---

# Pattern Index

This index catalogs all architectural and design patterns used throughout the Hyperswitch codebase, providing a comprehensive reference for understanding the system's design principles.

## Architectural Patterns

### Layered Architecture
- **Description**: The system is organized into distinct layers with clear responsibilities
- **Implementation**: 
  - API Layer (Routes & Handlers)
  - Business Logic Layer (Core)
  - Data Access Layer (Storage)
- **Benefits**: Separation of concerns, maintainability, testability
- **Documentation**: [System Patterns](../systemPatterns.md), [Router Architecture](../thematic/crates/router/architecture/code_structure.md)

### Microservice Components
- **Description**: While primarily a monolith, the system uses microservice-like components
- **Implementation**: 
  - Scheduler service
  - Drainer service
  - Connector services
- **Benefits**: Scalability, fault isolation, targeted deployment
- **Documentation**: [Scheduler Overview](../thematic/crates/scheduler/overview.md), [Drainer Overview](../thematic/crates/drainer/overview.md)

### Event-Driven Architecture
- **Description**: Components communicate through events for loose coupling
- **Implementation**: 
  - Events crate for event definitions and handling
  - Event publishing and subscription
- **Benefits**: Decoupling, scalability, flexibility
- **Documentation**: [Events Overview](../thematic/crates/events/overview.md)

### Domain-Driven Design
- **Description**: Modeling based on the business domain
- **Implementation**: 
  - Domain models in hyperswitch_domain_models
  - Bounded contexts for payment processing, refunds, etc.
- **Benefits**: Alignment with business needs, expressive models
- **Documentation**: [Domain Models](../thematic/crates/hyperswitch_domain_models/overview.md)

## Design Patterns

### Repository Pattern
- **Description**: Abstraction over data storage
- **Implementation**: 
  - Repository implementations in storage_impl
  - Data access interfaces
- **Benefits**: Decoupling from database, testability
- **Documentation**: [Storage Implementation](../thematic/crates/storage_impl/overview.md)

### Strategy Pattern
- **Description**: Family of algorithms, encapsulated and interchangeable
- **Implementation**: 
  - Connector implementations
  - Routing strategies
- **Benefits**: Runtime flexibility, extension without modification
- **Documentation**: [Routing Strategies](../thematic/crates/router/configuration/routing_strategies.md), [Connector Interfaces](../thematic/crates/hyperswitch_interfaces/overview.md)

### Factory Pattern
- **Description**: Creating objects without specifying concrete classes
- **Implementation**: 
  - Connector factories
  - Payment method factories
- **Benefits**: Encapsulation, flexibility
- **Documentation**: [Hyperswitch Interfaces](../thematic/crates/hyperswitch_interfaces/overview.md)

### Adapter Pattern
- **Description**: Converting interfaces to be compatible
- **Implementation**: 
  - Connector transformers
  - API transformations
- **Benefits**: Reuse, interface compatibility
- **Documentation**: [Hyperswitch Connectors](../thematic/crates/hyperswitch_connectors/overview.md)

### Builder Pattern
- **Description**: Step-by-step construction of complex objects
- **Implementation**: 
  - Request builders
  - Configuration builders
- **Benefits**: Construction flexibility, parameter validation
- **Documentation**: Found throughout the codebase

### Decorator Pattern
- **Description**: Adding functionality to objects dynamically
- **Implementation**: 
  - Middleware components
  - Request/response wrappers
- **Benefits**: Open/closed principle, composition over inheritance
- **Documentation**: [Router Middleware](../thematic/crates/router/modules/middleware.md)

### Observer Pattern
- **Description**: Notify subscribers of state changes
- **Implementation**: 
  - Event system
  - Webhook notifications
- **Benefits**: Loose coupling, extensibility
- **Documentation**: [Events Overview](../thematic/crates/events/overview.md), [Webhook Flows](../thematic/crates/router/flows/webhook_flows.md)

## Domain-Specific Patterns

### Payment Processor Abstraction
- **Description**: Unified interface for multiple payment processors
- **Implementation**: 
  - Connector trait implementations
  - Standardized request/response transformations
- **Benefits**: Processor interchangeability, consistent handling
- **Documentation**: [Hyperswitch Interfaces](../thematic/crates/hyperswitch_interfaces/overview.md), [Connector Integration](../thematic/crates/hyperswitch_interfaces/connector_integration.md)

### Payment Flow Orchestration
- **Description**: Coordinating multi-step payment processes
- **Implementation**: 
  - Payment operations
  - State-based flow progression
- **Benefits**: Complex flow management, error handling
- **Documentation**: [Payment Flows](../thematic/crates/router/flows/payment_flows.md)

### Intelligent Payment Routing
- **Description**: Dynamic selection of payment processors
- **Implementation**: 
  - Routing rules DSL (Euclid)
  - Constraint-based routing decisions
- **Benefits**: Optimized processing, dynamic adaptation
- **Documentation**: [Routing Strategies](../thematic/crates/router/configuration/routing_strategies.md), [Euclid](../thematic/crates/euclid/overview.md)

### Payment Method Validation
- **Description**: Standardized validation for payment methods
- **Implementation**: 
  - Card validation
  - Payment method-specific validation
- **Benefits**: Consistency, error prevention
- **Documentation**: [Cards](../thematic/crates/cards/overview.md), [Payment Methods](../thematic/crates/payment_methods/overview.md)

## Concurrency Patterns

### Actor Model (Partial)
- **Description**: Concurrent computation with isolated state
- **Implementation**: 
  - Scheduler worker processes
  - Task-based processing
- **Benefits**: Concurrency, fault isolation
- **Documentation**: [Scheduler](../thematic/crates/scheduler/overview.md)

### Producer-Consumer
- **Description**: Separate production and consumption of data
- **Implementation**: 
  - Scheduler producer/consumer components
  - Queue-based processing
- **Benefits**: Workload distribution, decoupling
- **Documentation**: [Scheduler](../thematic/crates/scheduler/overview.md)

## Integration Patterns

### Gateway Pattern
- **Description**: Single entry point for external systems
- **Implementation**: 
  - Router API as a payment gateway
  - Uniform interface for multiple backends
- **Benefits**: Abstraction, security, monitoring
- **Documentation**: [Router Overview](../thematic/crates/router/overview.md)

### Anti-Corruption Layer
- **Description**: Translation between disparate models
- **Implementation**: 
  - Connector transformers
  - Domain model conversions
- **Benefits**: Model integrity, isolation
- **Documentation**: [Hyperswitch Connectors](../thematic/crates/hyperswitch_connectors/overview.md)

### Webhook Pattern
- **Description**: HTTP callbacks for asynchronous communication
- **Implementation**: 
  - Webhook handling flows
  - Event-driven notifications
- **Benefits**: Asynchronous updates, integration flexibility
- **Documentation**: [Webhook Flows](../thematic/crates/router/flows/webhook_flows.md)

## Data Management Patterns

### Repository Pattern
- **Description**: Abstraction over data storage
- **Implementation**: 
  - Storage implementations
  - Database access abstractions
- **Benefits**: Storage independence, testability
- **Documentation**: [Storage Implementation](../thematic/crates/storage_impl/overview.md)

### Caching Pattern
- **Description**: Temporary storage for performance
- **Implementation**: 
  - Redis-based caching
  - In-memory caching
- **Benefits**: Performance, scalability
- **Documentation**: [Redis Interface](../thematic/crates/redis_interface/overview.md)

### Data Masking Pattern
- **Description**: Secure handling of sensitive data
- **Implementation**: 
  - PII masking
  - Card data protection
- **Benefits**: Security, compliance
- **Documentation**: [Masking](../thematic/crates/masking/overview.md)

## Testing Patterns

### Test Fixtures
- **Description**: Reusable test setup components
- **Implementation**: 
  - Test utilities
  - Fixture generators
- **Benefits**: Test consistency, readability
- **Documentation**: [Test Utils](../thematic/crates/test_utils/overview.md)

### Mocking
- **Description**: Replacing dependencies in tests
- **Implementation**: 
  - Mock connectors
  - Test doubles
- **Benefits**: Isolation, controlled testing
- **Documentation**: [Test Utils](../thematic/crates/test_utils/overview.md)

## Related Resources

- [System Patterns](../systemPatterns.md) - Core architectural patterns
- [Router Architecture](../thematic/crates/router/architecture/code_structure.md) - Router code organization
- [Global Topic Index](./global_topic_index.md) - Index of all topics
- [Crate Functionality Index](./crate_functionality_index.md) - Index of crates by functionality
