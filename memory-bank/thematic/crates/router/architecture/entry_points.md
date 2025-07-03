---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

# Router Entry Points

---
**Parent:** [Router Overview](../overview.md)  
**Related Files:**
- [Code Structure](./code_structure.md)
- [Dependencies](./dependencies.md)
---

[← Back to Router Overview](../overview.md)

## Overview

The router crate defines two main binary entry points, each serving distinct purposes in the Hyperswitch platform. These entry points provide the starting points for the primary services that make up the application. This document details these entry points, their responsibilities, and their initialization processes.

## Main Binary Entry Points

### Router API Server (`src/bin/router.rs`)

The `router.rs` binary file serves as the entry point for the main Hyperswitch API server:

#### Purpose

- Initializes and runs the HTTP API server that processes payment requests
- Handles real-time API operations for payments, refunds, webhooks, etc.
- Implements the merchant-facing and admin APIs
- Processes synchronous operations in the platform

#### Initialization Process

1. **Configuration Loading**:
   - Loads environment variables
   - Reads configuration files (TOML)
   - Validates configuration values
   - Sets up feature flags based on compilation options

2. **Logging Setup**:
   - Initializes the logging framework (via `router_env`)
   - Configures log levels, formats, and destinations
   - Sets up request/response logging

3. **Database Connection**:
   - Establishes connection to the database
   - Sets up connection pools
   - Performs health check queries
   - Optionally runs migrations

4. **Redis Connection**:
   - Establishes connection to Redis
   - Configures Redis connection parameters
   - Sets up Redis pools

5. **Metrics Initialization**:
   - Sets up metrics collection
   - Configures exporters (Prometheus, etc.)
   - Initializes metric counters, gauges, and histograms

6. **Service Initialization**:
   - Initializes core services required by the application
   - Sets up dependency injection graph
   - Prepares service interfaces and implementations

7. **Middleware Configuration**:
   - Configures Actix Web middleware
   - Sets up authentication, logging, metrics middleware
   - Configures CORS settings
   - Sets up error handling

8. **Route Registration**:
   - Registers all API routes with the Actix Web application
   - Configures route groups and prefixes
   - Sets up versioned API endpoints
   - Configures health check endpoints

9. **Server Startup**:
   - Binds to configured network interface and port
   - Configures worker threads
   - Sets up graceful shutdown handling
   - Starts the HTTP server

#### Command-Line Arguments

The router binary supports several command-line arguments:

- **`--config`**: Path to the configuration file
- **`--log-level`**: Overrides the log level (debug, info, warn, error)
- **`--bind-address`**: Overrides the server binding address
- **`--port`**: Overrides the server port
- **`--workers`**: Number of worker threads to use

#### Usage Example

```bash
# Start with default settings
cargo run --bin router

# Start with a specific configuration file
cargo run --bin router -- --config config/production.toml

# Override specific settings
cargo run --bin router -- --port 8080 --log-level debug
```

### Scheduler Service (`src/bin/scheduler.rs`)

The `scheduler.rs` binary file serves as the entry point for the Hyperswitch Scheduler service:

#### Purpose

- Processes asynchronous, background, and scheduled tasks
- Manages recurring operations (retries, status checks, etc.)
- Handles long-running operations that don't fit in the API request/response cycle
- Executes maintenance tasks

#### Initialization Process

1. **Configuration Loading**:
   - Similar to the router binary
   - May have scheduler-specific configuration options

2. **Logging Setup**:
   - Similar to the router binary
   - May use different log prefixes or identifiers

3. **Database Connection**:
   - Similar to the router binary
   - May use read-only connections for some operations

4. **Redis Connection**:
   - Similar to the router binary
   - Configures Redis for task queue management

5. **Task Configuration**:
   - Registers task types and their handlers
   - Configures task priorities and concurrency limits
   - Sets up task scheduling rules

6. **Scheduler Initialization**:
   - Sets up the task scheduler engine
   - Configures worker threads
   - Initializes task queues
   - Sets up periodic tasks

7. **Worker Thread Pool**:
   - Creates a thread pool for executing tasks
   - Configures thread pool size and behavior
   - Sets up task distribution mechanisms

8. **Service Startup**:
   - Starts the scheduler service
   - Begins processing tasks
   - Sets up graceful shutdown handling

#### Core Task Types

The scheduler typically handles several task types:

- **Payment Status Sync**: Periodically checks payment status with processors
- **Refund Status Sync**: Checks refund status with processors
- **Webhook Delivery**: Manages outgoing webhook delivery attempts and retries
- **Data Cleanup**: Handles cleanup of old or unnecessary data
- **Metrics Collection**: Gathers and aggregates metrics for reporting

#### Command-Line Arguments

The scheduler binary supports several command-line arguments:

- **`--config`**: Path to the configuration file
- **`--log-level`**: Overrides the log level (debug, info, warn, error)
- **`--concurrency`**: Sets the maximum number of concurrent tasks
- **`--task-types`**: Comma-separated list of task types to process (default: all)

#### Usage Example

```bash
# Start with default settings
cargo run --bin scheduler

# Start with a specific configuration file
cargo run --bin scheduler -- --config config/scheduler.toml

# Process only specific task types
cargo run --bin scheduler -- --task-types payment_sync,webhook_delivery
```

## Integration Between Entry Points

The router and scheduler binaries work together as part of the Hyperswitch platform:

- **Shared Configuration**: Both use the same configuration framework
- **Shared Database**: Both access the same database, with appropriate transactions
- **Task Queuing**: Router may enqueue tasks for the scheduler
- **State Management**: Both update and read from shared state
- **Service Discovery**: May use service discovery to locate each other in distributed deployments

## Development and Testing

For development and testing purposes, both binaries can be run locally:

### Development Mode

```bash
# Start the router in development mode
cargo run --bin router -- --config config/development.toml

# Start the scheduler in development mode
cargo run --bin scheduler -- --config config/development.toml
```

### Testing

Each binary has its own test suite:

- Unit tests for individual components
- Integration tests for end-to-end functionality
- Mock services for external dependencies

## Deployment Considerations

When deploying to production, several considerations apply:

- **Scaling**: Router instances can scale horizontally behind a load balancer
- **Redundancy**: Multiple scheduler instances can run with task locking to prevent duplication
- **Monitoring**: Both binaries expose health check and metrics endpoints
- **Resource Allocation**: Router typically needs more CPU, scheduler may need more memory
- **Networking**: Router needs public access, scheduler typically internal only

## Document History
| Date | Changes |
|------|---------|
| 2025-05-27 | Updated to new documentation standard format |
| 2025-05-20 | Last content update before standardization |
| Prior | Initial version |

## See Also

- [Code Structure Documentation](./code_structure.md)
- [Dependencies Documentation](./dependencies.md)
