---
title: Configuration Option Index
last_updated: 2025-05-27
position: 1
---

# Configuration Option Index

This index catalogs all configuration options available in the Hyperswitch system, including environment variables, feature flags, configuration file settings, and other configurable parameters.

## Core Configuration

### Environment Variables

These environment variables control core system behavior:

- `SERVER_BASE_URL` - Base URL for the server
- `API_KEY` - API key for authentication
- `LOCKER_API_KEY` - API key for the card locker service
- `PORT` - Port on which the server listens
- `HOST` - Host address for the server
- `LOG_LEVEL` - Logging level (debug, info, warn, error)
- `LOG_FORMAT` - Format for logs (json, plain)
- `RUST_LOG` - Rust-specific logging configuration
- `RUST_BACKTRACE` - Enable/disable backtraces
- **Documentation**: [Router Environment](../thematic/crates/router_env/overview.md)

### Feature Flags

These flags enable or disable specific features:

- `API_RATE_LIMITER` - Enable API rate limiting
- `API_ACCESS_RULES` - Enable API access rules
- `CONNECTOR_CHOICE` - Enable dynamic connector choice
- `CARDS_DOMAIN` - Enable cards domain functionality
- `OPEN_TELEMETRY` - Enable OpenTelemetry integration
- `MULTI_MERCHANT_SUPPORT` - Enable multi-merchant support
- `BACKWARD_COMPATIBILITY` - Enable backward compatibility mode
- **Documentation**: [Feature Flags](../thematic/crates/router/configuration/feature_flags.md)

### Database Configuration

Settings for database connections:

- `DATABASE_URL` - PostgreSQL connection string
- `DATABASE_POOL_SIZE` - Connection pool size
- `DATABASE_CONNECTION_TIMEOUT` - Connection timeout in seconds
- `DATABASE_PREPARE_STATEMENT_CACHE_SIZE` - Prepared statement cache size
- `DATABASE_MASTER_POOL_SIZE` - Master connection pool size
- `DATABASE_REPLICA_POOL_SIZE` - Replica connection pool size
- **Documentation**: [Storage Implementation](../thematic/crates/storage_impl/overview.md)

### Redis Configuration

Settings for Redis connections:

- `REDIS_HOST` - Redis host address
- `REDIS_PORT` - Redis port
- `REDIS_USERNAME` - Redis username
- `REDIS_PASSWORD` - Redis password
- `REDIS_POOL_SIZE` - Connection pool size
- `REDIS_CONN_TIMEOUT` - Connection timeout in seconds
- `REDIS_CLUSTER_ENABLED` - Enable Redis cluster mode
- `REDIS_CLUSTER_URLS` - Comma-separated list of cluster URLs
- **Documentation**: [Redis Interface](../thematic/crates/redis_interface/overview.md)

## TOML Configuration File Settings

The following settings are configured in the TOML configuration files (development.toml, production.toml, etc.):

### Server Settings

```toml
[server]
host = "127.0.0.1"
port = 8080
workers = 10
shutdown_timeout = 30
request_body_limit = "10MB"
cors_allow_origin = ["*"]
base_url = "http://localhost:8080"
```

### Database Settings

```toml
[database]
username = "db_user"
password = "db_password"
host = "localhost"
port = 5432
dbname = "hyperswitch"
pool_size = 5
connection_timeout = 10
```

### Redis Settings

```toml
[redis]
host = "localhost"
port = 6379
pool_size = 5
connection_timeout = 10
reconnect_max_attempts = 5
reconnect_delay = 5
```

### Locker Settings

```toml
[locker]
host = "localhost"
port = 8081
mock_locker = true
```

### Connector Settings

```toml
[connectors]
aci.base_url = "https://api.example.com"
adyen.base_url = "https://checkout-test.adyen.com"
airwallex.base_url = "https://api-demo.airwallex.com"
```

**Documentation**: [Configuration Files](../config/development.toml)

## Connector Configuration

### Connector Credentials

Configuration for payment processor connections:

- API keys
- Merchant IDs
- Base URLs
- Webhook secrets
- Timeout settings
- **Documentation**: [Connector Configs](../thematic/crates/connector_configs/overview.md)

### Routing Configuration

Settings for payment routing:

- Priority configuration
- Fallback rules
- Routing conditions
- Connector eligibility
- **Documentation**: [Routing Strategies](../thematic/crates/router/configuration/routing_strategies.md)

## Scheduler Configuration

Settings for the task scheduler:

- `SCHEDULER_INTERVAL` - Interval between scheduler runs
- `SCHEDULER_CONSUMER_COUNT` - Number of consumer workers
- `SCHEDULER_BUFFER_SIZE` - Scheduler buffer size
- `TASK_RETRY_COUNT` - Maximum number of task retries
- `TASK_RETRY_INTERVAL` - Interval between task retries
- **Documentation**: [Scheduler](../thematic/crates/scheduler/overview.md)

## Security Configuration

### Authentication Settings

- `JWT_SECRET` - Secret key for JWT tokens
- `JWT_EXPIRY` - JWT token expiry time
- `API_KEY_PREFIX` - Prefix for API keys
- `ADMIN_API_KEY` - Admin API key
- **Documentation**: [Router Middleware](../thematic/crates/router/modules/middleware.md)

### Data Protection Settings

- `CARD_DATA_MASKING` - Enable card data masking
- `PII_MASKING` - Enable PII masking
- `MASKING_LEVEL` - Masking strictness level
- **Documentation**: [Masking](../thematic/crates/masking/overview.md)

## Logging and Monitoring

### Logging Configuration

- `LOG_FILE` - Log file path
- `LOG_ROTATION` - Log rotation policy
- `LOG_FORMAT` - Log format (JSON, plain text)
- `LOG_LEVEL` - Log level (debug, info, warn, error)
- **Documentation**: [Router Environment](../thematic/crates/router_env/overview.md)

### Monitoring Configuration

- `METRICS_ENABLED` - Enable metrics collection
- `METRICS_PORT` - Port for metrics server
- `METRICS_HOST` - Host for metrics server
- `PROMETHEUS_ENABLED` - Enable Prometheus integration
- **Documentation**: [Router Environment](../thematic/crates/router_env/overview.md)

## Performance Tuning

### Connection Pooling

- `HTTP_CLIENT_TIMEOUT` - HTTP client timeout
- `HTTP_CLIENT_CONNECTION_TIMEOUT` - HTTP connection timeout
- `HTTP_CLIENT_POOL_SIZE` - HTTP client connection pool size
- `HTTP_CLIENT_POOL_IDLE_TIMEOUT` - HTTP client pool idle timeout
- **Documentation**: [Router Environment](../thematic/crates/router_env/overview.md)

### Caching Configuration

- `CACHE_ENABLED` - Enable caching
- `CACHE_TTL` - Cache time-to-live
- `CACHE_CAPACITY` - Cache capacity
- **Documentation**: [Redis Interface](../thematic/crates/redis_interface/overview.md)

## Developer Configuration

### Test Mode Settings

- `TEST_MODE` - Enable test mode
- `MOCK_CONNECTORS` - Use mock connectors
- `MOCK_LOCKER` - Use mock locker
- **Documentation**: [Test Utils](../thematic/crates/test_utils/overview.md)

### Development Utilities

- `SEED_DATA` - Create seed data on startup
- `LIVE_RELOAD` - Enable live reloading
- `DEV_AUTH_ENABLED` - Enable development authentication
- **Documentation**: [HSdev](../thematic/crates/hsdev/overview.md)

## Configuration File Locations

- `/config/development.toml` - Development environment configuration
- `/config/production.toml` - Production environment configuration
- `/config/dashboard.toml` - Dashboard configuration
- `/config/docker_compose.toml` - Docker Compose configuration
- `/config/payment_required_fields_v2.toml` - Payment required fields configuration

## Configuration Management

### Configuration Loading Precedence

1. Environment variables
2. Command-line arguments
3. Configuration files
4. Default values

### Dynamic Configuration

- Feature flags for runtime feature toggling
- Hot reloading of certain configuration parameters
- **Documentation**: [Router Environment](../thematic/crates/router_env/overview.md)

## Related Resources

- [Global Topic Index](./global_topic_index.md) - Complete topic index
- [Crate Functionality Index](./crate_functionality_index.md) - Crate index by functionality
- [Pattern Index](./pattern_index.md) - Design pattern index
- [API Index](./api_index.md) - API reference index
