# Active Context

## Current Focus

- Integrating the detailed "Hyperswitch Connector Integration Assistant" guide into the Memory Bank, enhancing the existing documentation for adding new connectors.

## Recent Changes

- Updated `techContext.md` with the comprehensive "Hyperswitch Connector Integration Assistant" guide, including detailed steps for payment method selection, flow selection, API documentation analysis, amount type specification, connector body creation (field compilation, body generation), type discovery rules, struct generation phases, the `flow_guide` for card payments, and Hyperswitch-specific request/response context. Minor clarifications regarding template usage and amount conversion patterns were also added based on code verification.
- Updated `systemPatterns.md` to reflect the new guided integration methodology, flow selection logic, and structured implementation phases.

## Next Steps

- Update `progress.md` to reflect the enhanced connector integration documentation.
- Review `projectbrief.md` and `productContext.md` for any potential high-level updates, though current detailed technical changes might not directly impact them.
- Await further instructions or tasks.

## Key Decisions & Considerations

- [Document active decisions and important considerations]

## Important Patterns & Preferences

- [Note any emerging patterns or user preferences]

## Learnings & Insights

- Gained a comprehensive understanding of the connector architecture, including the role of `hyperswitch_connectors` crate, `common_enums` for connector registration, the structure of `transformers.rs` and the main connector logic file, testing procedures, and configuration in both backend and the Control Center.
- Deepened understanding of the structured approach to connector integration through the "Hyperswitch Connector Integration Assistant" guide. This includes:
    - The importance of a phased approach: API analysis, type discovery, struct generation, and transformer implementation.
    - Specific rules for type handling in Hyperswitch (e.g., using `pii::Email`, `cards::CardNumber`, `masking::Secret`, `enums::CountryAlpha2`, `api_models::Currency`, `serde` attributes like `skip_serializing_if = "Option::is_none"` and `rename_all`).
    - The utility of a `flow_guide` for making informed decisions about which Hyperswitch payment/authorization flow to implement based on connector capabilities.
    - The critical role of `crates/hyperswitch_domain_models/src/router_request_types.rs` and `crates/hyperswitch_domain_models/src/router_response_types.rs` as the source of truth for Hyperswitch's internal data structures.
    - Verification against actual connector code (e.g., `stripebilling`, `connector-template`) confirms high consistency between documentation and implementation, with template files providing excellent starting points including `TODO` comments for developer guidance.
