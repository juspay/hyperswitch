# System Patterns

*   **Architecture Overview:** Modular, service-oriented architecture primarily consisting of:
    *   **Router:** Main synchronous request processing engine for payment flows. Handles API requests, authentication, core logic execution, and interaction with other components.
    *   **Scheduler:** Asynchronous background task processor (Producer/Consumer pattern using Redis queue).
    *   **Locker:** Dedicated secure vault for PII/PCI data.
    *   **Database:** PostgreSQL (Master/Replica) for persistent state.
    *   **Cache/Queue:** Redis.
    *   **Configuration System:** Fully migrated to support Superposition's Context-Aware-Configuration (CAC) with fallback to TOML files. Complete implementation in `crates/router/src/configs/` with domain-specific modules for all configuration areas.
    *   **Frontend:** SDKs (Web, Mobile) and Control Center (Web UI).
    *   **Monitoring Stack:** OTel Collector, Prometheus, Loki, Tempo, Grafana.
*   **Key Technical Decisions:**
    *   Rust as the primary backend language (performance, safety).
    *   Modular design using Rust crates.
    *   Separation of concerns between synchronous (Router) and asynchronous (Scheduler) processing.
    *   Dedicated secure component (Locker) for sensitive data.
    *   Use of standard, widely adopted database and caching technologies (Postgres, Redis).
    *   Complete integration with Superposition's Context-Aware-Configuration (CAC). All configuration domains now support dynamic, context-aware resolution based on merchant/profile/region dimensions with graceful fallback to TOML.
    *   OpenTelemetry for standardized observability.
    *   **`MerchantContext` Enum:** Centralized in the authentication layer (`AuthenticationData`) to unify context handling for standard merchant operations and platform/connected account scenarios. Instantiated via `From<(MerchantAccount, MerchantKeyStore)>`.
*   **Design Patterns:**
    *   Service-Oriented Architecture.
    *   Producer/Consumer (for Scheduler).
    *   Repository/Data Access Layer (implied by Diesel usage).
    *   Connector/Adapter pattern (for integrating with PSPs).
    *   **Context Object Pattern:** The `MerchantContext` enum acts as a context object holding relevant merchant/platform details derived during authentication.
    *   **Strategy Pattern:** Used extensively in CAC integration where different configuration resolvers handle domain-specific settings (database, redis, connectors, etc.) with context-aware optimizations.
    *   **Decorator/Wrapper Pattern:** The enhanced `HyperswitchCacClient` with caching wraps the core CAC client, adding performance optimizations and resilience.
    *   **Factory Pattern:** Configuration modules create context-optimized defaults based on merchant type (enterprise/startup/marketplace) and region (EU/US/APAC).
    *   **Newtype Pattern:** The `router_env::logger::config::Level` is a newtype wrapper around `tracing::Level` to provide a custom `Deserialize` implementation suitable for configuration files, while still leveraging the core `tracing::Level` functionality.
*   **Component Relationships:**
    *   SDKs/API -> Router (Authentication -> `MerchantContext` creation) -> (Connectors | Locker | DB | Scheduler)
    *   Control Center -> Router API
    *   Router -> `ConfigResolver` -> `HyperswitchCacClient` -> Superposition CAC Service -> Context-Aware Configuration Resolution with TOML Fallback.
    *   Scheduler Producer -> DB (read tasks) -> Redis (queue tasks)
    *   Scheduler Consumer -> Redis (dequeue tasks) -> Router (execute task logic) -> DB (update task status)
    *   All services -> OTel Collector -> Monitoring Backend (Prometheus, Loki, Tempo)
*   **Critical Implementation Paths:**
    *   Core payment processing flow within the Router.
    *   **Authentication flow:** Verifying credentials (API keys, JWTs) and centrally constructing the `MerchantContext` within `AuthenticationData`.
    *   Downstream usage of `auth.merchant_context` in route handlers and core logic to access the correct merchant account and key store.
    *   **Configuration resolution:** Comprehensive CAC-first approach with TOML fallback. The `ConfigResolver` builds context from request (merchant_id, profile_id, region), queries CAC for context-aware settings, applies merchant-type and regional optimizations, and falls back to static TOML on CAC unavailability. Supports hot reload for runtime updates.
    *   Connector integrations.
    *   Secure data handling in the Locker.
    *   Task scheduling reliability.
