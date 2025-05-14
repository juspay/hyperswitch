# System Patterns

## System Architecture

- [Describe the overall system architecture. A diagram might be useful here later.]

## Key Technical Decisions

- [List key technical decisions made so far]

## Design Patterns

- [Identify any design patterns in use or planned]

### Connector Integration Pattern

- **Location**: New connectors are implemented within the `crates/hyperswitch_connectors/src/connectors/` directory.
- **Structure**: Each connector typically consists of:
    - A main logic file: `<connector_name>.rs` (e.g., `billwerk.rs`).
    - A transformers file: `<connector_name>/transformers.rs` for request/response struct definitions and data mapping.
- **Implementation**: Integration is trait-based. Key traits include:
    - `ConnectorCommon`: For basic info like ID, base URL, auth.
    - `ConnectorIntegration<Flow, Request, Response>`: For each payment/refund flow (Authorize, Capture, Sync, Refund, etc.).
    - `ConnectorSpecifications`: For metadata like supported payment methods.
- **Registration**: New connectors must be added as variants to the `Connector` enum (and `RoutableConnectors` if applicable) located in `crates/common_enums/src/connector_enums.rs`.
- **Statelessness**: Connector modules are designed to be stateless, with the core router handling data persistence.

## Component Relationships

- [Describe how major components interact]

## Critical Implementation Paths

- [Highlight any critical paths in the implementation]
