# Hyperswitch System Architecture and Patterns

## High-Level Architecture

Hyperswitch comprises two distinct app services: **Router** and **Scheduler**, along with supporting components like databases, a secure storage component (Locker), and monitoring services.

```mermaid
graph TD
    A[Client] --> B[Router]

    subgraph HyperswitchCoreAndStorage
        B --> C[Database - PostgreSQL]
        B --> UnifiedRedis[Redis (Cache & Queue)]
        B --> Locker[Locker (Secure Storage Component)]
        B --> F[Payment Processors]
        B --> G[Scheduler]
    end
    
    %% Locker is an internal component of the Router, or runs closely alongside it.
    %% The diagram shows it connected to the Router; textual description clarifies its nature.

    G --> UnifiedRedis
    G --> C // Scheduler also interacts with PostgreSQL for task data

    subgraph Monitoring
        J[OpenTelemetry Collector]
        K[Prometheus]
        L[Loki]
        M[Tempo]
        N[Grafana]
    end
    
    B --> J
    G --> J
```

## Core Components

### Router

The Router is the main component of Hyperswitch, serving as the primary crate where all the core payment functionalities are implemented. It is responsible for:

- Processing payment requests
- Routing payments to appropriate processors
- Managing payment flows (authorization, authentication, void, capture)
- Handling post-payment processes (refunds, chargebacks)
- Implementing payment routing strategies (success rate-based, rule-based, etc.)
- Fallback handling and retry mechanisms

### Scheduler

The Scheduler consists of two components:

1.  **Producer (Job Scheduler)**:
    -   Tracks tasks yet to be executed
    -   Retrieves tasks from the database when scheduled time is up
    -   Groups or batches tasks together
    -   Stores batches in the Redis queue for execution

2.  **Consumer (Job Executor)**:
    -   Retrieves batches of tasks from the Redis queue
    -   Executes the tasks according to required processing logic

Use cases include:
- Removing saved card information after a certain period
- Notifying merchants about API key expiration

### Database

#### PostgreSQL

- Stores customer information, merchant details, payment data.
- Uses a master-database and replica-database setup as an out-of-the-box configuration to optimize read and write operations.

#### Redis

- A single Redis instance typically serves two main purposes:
    -   **Caching** (primarily by the Router): Stores frequently accessed data to reduce latency and database load.
    -   **Queuing** (primarily by the Scheduler): Used for task management and asynchronous job processing.

### Locker (Secure Storage Component)

- An internal component, designed to run alongside the Router, providing GDPR-compliant PII storage and secure encryption.
- It is fully compliant with PCI DSS requirements, ensuring secure handling and storage of sensitive payment data.

### Monitoring

The monitoring architecture includes:
- **OpenTelemetry Collector**: Collects metrics and traces
- **Prometheus**: Retrieves application metrics from the collector
- **Promtail**: Scrapes application logs
- **Loki**: Stores logs
- **Tempo**: Queries application traces
- **Grafana**: Visualizes metrics, logs, and traces

## Key Design Patterns

### Modular Architecture

Hyperswitch is organized into multiple crates, each with specific responsibilities:

-   **`router`**: Core payment processing logic and API handling.
-   **`scheduler`**: Task scheduling and execution.
-   **`storage_impl`**: Database interaction layer abstracting database operations.
-   **`diesel_models`**: Provides ORM capabilities and defines database schema structures using Diesel.
-   **`hyperswitch_connectors`**: Handles integrations with various payment processors.
-   **`api_models`**: Defines API request and response structures.
-   **`common_enums`**: Contains shared enumerations used across the system.
-   **`common_utils`**: Provides common utility functions and helpers.

### Payment Flow Patterns

1.  **Authorization Flow**: Validates and authorizes payment.
2.  **Authentication Flow**: Handles 3DS and other authentication methods.
3.  **Capture Flow**: Captures authorized payments.
4.  **Void Flow**: Cancels authorized payments.
5.  **Refund Flow**: Processes refunds for captured payments.

### Connector Integration Pattern

Hyperswitch uses a standardized approach for integrating payment processors:

1.  **Connector Trait** (e.g., `trait Connector`): Defines the standard Rust trait that all specific payment processor integrations must implement.
2.  **Transformers**: Convert between Hyperswitch's internal models and connector-specific data formats.
3.  **Routing Logic**: Determines which connector to use based on various criteria (rules, cost, success rate).
4.  **Fallback Mechanism**: Handles failures by attempting transactions with alternative connectors if configured.

### Error Handling Pattern

- Comprehensive error types for different failure scenarios.
- Structured error responses with appropriate HTTP status codes.
- Retry mechanisms based on error types and connector responses.
- Extensive logging and monitoring of errors for analysis and alerting.

### Database Interaction Pattern

- Uses Diesel ORM for type-safe SQL queries and schema management.
- Employs a repository pattern (within `storage_impl`) for data access abstraction.
- Utilizes connection pooling for efficient database usage.
- Manages transactions to ensure atomicity for critical operations.

## Cross-Cutting Concerns

### Security

- PCI DSS compliance for payment data handling.
- Encryption of sensitive data at rest and in transit, leveraging the Locker component for PII.
- Secure API authentication and authorization mechanisms.
- Input validation and sanitization to prevent common vulnerabilities.

### Performance Optimization

- Caching of frequently accessed data in Redis.
- Asynchronous processing for non-critical tasks and I/O operations.
- Database query optimization and efficient indexing.
- Connection pooling for database and external service interactions.

### Observability

- Structured logging across all components.
- Comprehensive metrics collection using OpenTelemetry.
- Distributed tracing to monitor request flows across services.
- Alerting on critical issues based on metrics and logs.

## Links to Detailed Documentation

- [Router Architecture](./thematic/crates/router/architecture.md)
- [Scheduler Architecture](./thematic/crates/scheduler/architecture.md)
- [Database Schema](./thematic/database/schema.md)
- [Connector Integration Guide](./thematic/connectors/integration_guide.md)
- [Payment Flows](./thematic/payment_flows/overview.md)