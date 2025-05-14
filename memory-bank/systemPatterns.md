# System Patterns

## System Architecture

- [Describe the overall system architecture. A diagram might be useful here later.]

## Key Technical Decisions

- [List key technical decisions made so far]

## Design Patterns

- [Identify any design patterns in use or planned]
- **Guided Integration Assistant**: A structured, step-by-step methodology for integrating new connectors, emphasizing clear phases for API understanding, type discovery, struct generation, and flow selection (see `techContext.md` for full details).

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
- **Guided Flow Selection**: The integration process now includes a `flow_guide` (detailed in `techContext.md`) to help determine the appropriate Hyperswitch flow (e.g., DirectAuthorization, PreprocessingBasedAuthorization) based on the connector's API capabilities for specific payment methods like Cards. This involves comparing API request/response formats with Hyperswitch's requirements.
- **Structured Implementation Phases**: The "Integration Assistant" model promotes:
    - **Type Discovery**: Systematically identifying and mapping connector-specific types to Hyperswitch types, including enums, security handling for sensitive data (e.g., `pii::Email`, `cards::CardNumber`, `masking::Secret`), and proper serialization attributes (`serde`).
    - **Struct Generation**: Creating well-defined request and response structs for the connector, adhering to Hyperswitch conventions.
    - **Transformer Implementation**: Centralizing data conversion logic within `transformers.rs` using `TryFrom` traits to map between Hyperswitch's `RouterData` and connector-specific structs.

## Component Relationships

- [Describe how major components interact]

## Critical Implementation Paths

- [Highlight any critical paths in the implementation]
