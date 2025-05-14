# Project Brief: Hyperswitch Codebase Analysis and Rulebook Generation

**Objective:** To analyze specific aspects of the Hyperswitch Rust codebase, focusing on connector integrations, payment flows, and code reuse patterns, and to document these findings in a "rulebook."

**Scope:**
- Analyze the process of adding new connectors by reviewing `add_connector_updated.md`.
- Examine the integration architecture of a selection of 5-10 payment connectors (e.g., Stripe).
- Identify and document common payment flows (authorize, capture, refund, sync, etc.) as implemented by these connectors.
- Investigate and document code reuse mechanisms, including default implementations, shared utilities, and trait usage (e.g., within `crates/hyperswitch_connectors`, `crates/hyperswitch_interfaces`).
- Create and maintain a `rulebook.md` file that serves as a living document, updated iteratively with insights specific to connector architecture, flows, and reuse.
- Maintain other Memory Bank files (`productContext.md`, `techContext.md`, `systemPatterns.md`, `activeContext.md`, `progress.md`) to support this focused analysis.

**Goals:**
- Produce a rulebook that helps developers (and myself) understand how to integrate and work with payment connectors in Hyperswitch.
- Document the standard payment flows and how they are handled across different connectors.
- Identify and explain key code reuse strategies to promote efficient and consistent development.
- Understand the general structure and requirements for connector modules.
