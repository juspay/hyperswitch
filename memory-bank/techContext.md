# Tech Context

*   **Primary Language(s):** Rust (Backend), TypeScript/JavaScript (likely for SDKs/Control Center).
*   **Frameworks/Libraries:**
    *   **Rust:** Actix (web framework), Diesel (ORM), Tokio (async runtime), Redis client, OpenTelemetry SDK, `serde` (serialization/deserialization), `serde_json` (JSON handling), `humantime-serde` (duration parsing).
        *   **CAC Integration:** Custom implementation in `crates/router/src/configs/` replacing initial `hyperswitch_cac` crate approach
        *   Key modules: `cac_client.rs` (enhanced client with caching), `cac_resolver.rs` (resolution framework)
    *   **Superposition:** Context-Aware-Configuration (CAC) fully integrated across all configuration domains
        *   Enhanced client with local caching for performance
        *   Fallback mechanism to static TOML configuration
        *   Hot reload capabilities for runtime updates
    *   **Frontend:** [To be confirmed - Likely React/Vue/Angular for Control Center/Web SDK]
*   **Database(s):** PostgreSQL (Primary), Redis (Cache & Queue).
*   **Key Dependencies:** 
    *   External Payment Service Providers (PSPs)
    *   Fraud Risk Management (FRM) services
    *   Authentication services (integrated via connectors)
    *   Superposition's CAC service for dynamic configuration management
    *   All configuration now supports context-aware resolution based on merchant/profile/region
*   **Development Environment:** Docker (via `docker-compose.yml` for local setup), `cargo` build system.
    *   **Project Structure:** Includes multiple crates within the workspace (e.g., `router`, `router_env`, `hyperswitch_cac`).
    *   **Tooling:** `rustfmt` (formatter), `clippy` (linter), `cargo` (build/test/run), Docker, Git. Cypress (E2E testing).
        *   **Diagnostics:** Rely heavily on `cargo check` and `just check_v2` (which likely wraps `cargo check` with specific features/targets) to identify compilation errors and guide refactoring, especially after significant changes or when iterating on fixes.
    *   **Technical Constraints:** PCI DSS compliance requirements for handling payment data. Need for high reliability, security, and performance.
