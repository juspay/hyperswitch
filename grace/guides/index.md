# Hyperswitch Connector Integration Guides

This directory contains guides and documentation to support the integration of payment connectors into the Hyperswitch system, with a particular focus on AI-assisted development workflows.

## Guide Index

### Core Integration Guides
- [`connector_integration_guide.md`](./connector_integration_guide.md) - Comprehensive step-by-step guide for integrating a new payment connector with AI assistance.

### Specialized Guides

#### Integration Patterns
- [`integrations/integrations.md`](./integrations/integrations.md) - Detailed documentation on structuring a new Hyperswitch connector, including explanations of the two-file structure (`<connector_name>.rs` and `<connector_name>/transformers.rs`).

#### System Patterns
- [`patterns/patterns.md`](./patterns/patterns.md) - Documentation on design patterns used in the Hyperswitch system, particularly connector integration patterns.

#### Error Handling
- [`errors/errors.md`](./errors/errors.md) - Guide to error handling in connector integrations, including common error patterns and best practices.

#### Type System
- [`types/types.md`](./types/types.md) - Documentation on type handling and conversions in Hyperswitch connector integrations.

#### Learning Resources
- [`learning/learning.md`](./learning/learning.md) - Additional learning resources for connector integration.

## Quick Start

If you're new to Hyperswitch connector integration, follow this sequence:

1. Start with the [Connector Integration Guide](./connector_integration_guide.md) for a complete overview of the process
2. Review [Integration Patterns](./integrations/integrations.md) to understand the file structure and best practices
3. Explore [System Patterns](./patterns/patterns.md) to learn about design patterns used across connectors
4. Reference [Type System](./types/types.md) and [Error Handling](./errors/errors.md) as needed during implementation

## For AI Assistants

AI assistants should pay particular attention to:
- The structured phases in the connector integration guide
- Type conversion patterns between Hyperswitch and connector-specific formats
- The separation of concerns between transformer files (data structures and conversion) and the main connector file (flow logic)
- Template usage guidelines in the `grace/connector_integration/template/` directory

All guides should be considered required context when planning or implementing a new connector integration.
