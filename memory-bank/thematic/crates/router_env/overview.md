---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

# Router Environment Crate Overview

The `router_env` crate provides essential environment management, logging, and metrics collection for the Hyperswitch payment orchestration platform. It serves as the central infrastructure component for operational awareness and observability, ensuring consistent logging, metrics collection, and environment detection across the system.

## Purpose

The `router_env` crate is responsible for:

1. Detecting and managing the runtime environment (development, sandbox, production)
2. Setting up structured logging with tracing
3. Configuring metrics collection and reporting
4. Providing version and build information
5. Supporting cargo workspace operations
6. Standardizing logging formats and categories

## Key Modules

### env.rs

The environment module provides environment detection and management:

- **Environment Detection**: Functions to determine the current environment (development, sandbox, production)
- **Environment Variables**: Definitions of environment variables used by the application
- **Path Management**: Functions to access workspace and configuration paths
- **Version Information**: Macros for retrieving build and version information

### logger.rs

The logging module provides a structured logging system using tracing:

- **Logging Setup**: Functions to set up and configure the logger
- **Log Categories**: Predefined categories for different types of logs
- **Log Tags**: Tags for identifying specific events or operations
- **Formatting**: Custom log formatting for structured output
- **Storage**: Optional log storage for later analysis

### metrics.rs

The metrics module provides utilities for OpenTelemetry metrics:

- **Metric Creation**: Macros for creating different types of metrics
- **Counter Metrics**: Utilities for tracking counts of events
- **Histogram Metrics**: Utilities for measuring distributions of values
- **Gauge Metrics**: Utilities for tracking values that can go up and down
- **Metric Attributes**: Macros for adding contextual attributes to metrics

### cargo_workspace.rs

Utilities for working with Cargo workspaces:

- **Workspace Detection**: Functions to detect the current workspace
- **Member Identification**: Utilities to identify workspace members
- **Path Manipulation**: Functions to get paths related to the workspace

## Core Features

### Environment Detection

The crate provides robust environment detection to adapt behavior based on the current environment:

- **Multiple Environments**: Support for development, sandbox, and production environments
- **Environment Variables**: Reading configuration from environment variables
- **Default Settings**: Sensible defaults based on build mode (debug or release)
- **Environment Prefixes**: Standard prefixes for each environment (dev, snd, prd)

### Structured Logging

A comprehensive logging system that enables detailed operational insights:

- **Tracing Integration**: Built on top of the tracing crate for powerful instrumentation
- **Structured Data**: Logs include structured metadata for easier analysis
- **Categories and Tags**: Predefined categories and tags for consistent log classification
- **Multi-level Logging**: Support for different log levels (debug, info, warn, error)
- **Context Propagation**: Ability to propagate context through spans

### Metrics Collection

Utilities for collecting and reporting metrics for monitoring:

- **OpenTelemetry Integration**: Built-in support for OpenTelemetry metrics
- **Multiple Metric Types**: Support for counters, histograms, and gauges
- **Metric Attributes**: Add contextual information to metrics
- **Global Meters**: Create global meters for consistent metric collection
- **Standard Buckets**: Predefined histogram buckets for consistent measurements

### Version and Build Information

Macros for accessing version and build information:

- **Version Information**: Access to git tags, commit hashes, and timestamps
- **Build Information**: Details about the build environment
- **Service Identification**: Utilities to identify the current service

## Usage Examples

### Environment Detection

```rust
use router_env::env::{self, Env};

// Check current environment
let current_env = env::which();
match current_env {
    Env::Development => println!("Running in development mode"),
    Env::Sandbox => println!("Running in sandbox mode"),
    Env::Production => println!("Running in production mode"),
}

// Get environment prefix
let prefix = env::prefix_for_env(); // "dev", "snd", or "prd"

// Get workspace path
let workspace = env::workspace_path();
```

### Logging

```rust
use router_env::logger;
use tracing::{instrument, Level};

// Set up the logger
let _guard = logger::setup(logger::Config::new("my_service"));

// Log with structured data
#[instrument]
fn process_payment(payment_id: &str) {
    logger::log!(
        Level::INFO,
        payment_id = payment_id,
        merchant_id = "merchant_123",
        tag = ?logger::Tag::PaymentProcessing,
        category = ?logger::Category::Processing,
        flow = "payment_flow",
        "Processing payment"
    );
}
```

### Metrics

```rust
use router_env::{counter_metric, global_meter, metric_attributes};

// Define a global meter
global_meter!(METER);

// Define metrics
counter_metric!(PAYMENT_COUNT, METER, "Number of payments processed");
histogram_metric_f64!(PAYMENT_AMOUNT, METER, "Distribution of payment amounts");

// Record metrics
fn record_payment(amount: f64, status: &str) {
    let attributes = metric_attributes![
        ("payment_status", status),
        ("payment_method", "card"),
    ];
    
    PAYMENT_COUNT.add(1, attributes);
    PAYMENT_AMOUNT.record(amount, attributes);
}
```

### Version Information

```rust
use router_env::{build, commit, version, service_name};

fn print_version_info() {
    println!("Service: {}", service_name!());
    println!("Version: {}", version!());
    println!("Commit: {}", commit!());
    println!("Build: {}", build!());
}
```

## Integration with Other Crates

The `router_env` crate is used throughout the Hyperswitch ecosystem:

1. **router**: Uses environment detection, logging, and metrics collection
2. **storage_impl**: Uses logging for database operations
3. **hyperswitch_connectors**: Uses logging for connector operations
4. **api_models**: Uses environment information for configuration
5. **drainer**: Uses logging and metrics for queue processing

## Performance Considerations

The crate implements several performance optimizations:

- **Lazy Initialization**: Uses `once_cell` for lazy initialization of meters and metrics
- **Efficient Environment Detection**: Caches environment detection results
- **Log Filtering**: Configurable log filtering to reduce overhead
- **Adaptive Logging Levels**: Adjust logging levels based on environment
- **Metric Sampling**: Support for sampling metrics in high-throughput scenarios

## Thread Safety and Async Support

The crate is designed for concurrent and async usage:

- **Thread-Safe Logging**: Safe for use in concurrent environments
- **Async-Compatible**: Works with async code and futures
- **Context Propagation**: Properly propagates context across async boundaries
- **Minimal Overhead**: Designed for minimal impact on performance

## Document History
| Date | Changes |
|------|---------|
| 2025-05-27 | Added metadata and document history section |
| Prior | Initial version |

## Conclusion

The `router_env` crate is a crucial infrastructure component of the Hyperswitch platform. It provides a consistent and powerful foundation for environment detection, logging, and metrics collection. Its integration with industry-standard tools like tracing and OpenTelemetry ensures that the platform has robust observability capabilities, making it easier to operate and troubleshoot in production environments.
