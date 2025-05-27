# OpenAPI Overview

The `openapi` crate generates OpenAPI specifications for the Hyperswitch API, providing comprehensive documentation for all API endpoints, data models, and authentication methods. This document provides an overview of its purpose, structure, and usage within the Hyperswitch ecosystem.

---
**Last Updated:** 2025-05-27  
**Documentation Status:** Complete
---

## Purpose

The `openapi` crate is responsible for:

1. Generating OpenAPI specifications for the Hyperswitch API
2. Documenting all API routes, endpoints, and data structures
3. Providing detailed descriptions for each API operation
4. Organizing API documentation by functional areas (tags)
5. Defining security schemes and authentication requirements

## Key Modules

The `openapi` crate is organized into the following key modules:

- **openapi.rs**: Core functionality for generating the OpenAPI specification
- **routes.rs**: Organizes API routes into logical categories
- **routes/**: Subdirectories containing route definitions for each API area:
  - payments
  - refunds
  - customers
  - mandates
  - merchant_account
  - merchant_connector_account
  - api_keys
  - disputes
  - payouts
  - webhook_events
  - and many others

## Core Features

### OpenAPI Specification Generation

The crate uses the `utoipa` crate to generate OpenAPI specifications from Rust code annotations:

- Detailed API endpoint documentation with descriptions, parameters, and examples
- Schema definitions for all request and response models
- Path grouping by logical categories
- Security scheme definitions
- Server environment configurations

### API Categorization

The API documentation is organized into functional categories (tags):

- Merchant Account - Create and manage merchant accounts
- Profile - Create and manage profiles
- Merchant Connector Account - Create and manage merchant connector accounts
- Payments - Create and manage one-time and recurring payments
- Refunds - Create and manage refunds for successful payments
- Mandates - Manage mandates
- Customers - Create and manage customers
- Payment Methods - Create and manage payment methods
- Disputes - Manage disputes
- API Key - Create and manage API Keys
- Payouts - Create and manage payouts
- Payment Link - Create payment links
- Routing - Create and manage routing configurations
- Event - Manage events

### Security Definition

The crate defines multiple security schemes for API authentication:

- api_key - For server-to-server API requests
- admin_api_key - For privileged admin operations
- publishable_key - For client-side API requests
- ephemeral_key - For temporary access to specific resources

## Public Interface

The crate's main public interface is the `ApiDoc` struct that implements the `utoipa::OpenApi` trait:

```rust
#[derive(utoipa::OpenApi)]
#[openapi(
    info(...),
    servers(...),
    tags(...),
    paths(...),
    components(schemas(...)),
    modifiers(&SecurityAddon)
)]
pub(crate) struct ApiDoc;
```

This structure contains all the OpenAPI documentation for the Hyperswitch API, with:

- API information and description
- Server configurations
- Tag definitions
- Path operations (endpoints)
- Component schemas (data models)
- Security scheme definitions

## Usage Examples

### Generating OpenAPI Specifications

```rust
// This is typically used within a main function to generate the OpenAPI specification
use openapi::ApiDoc;
use utoipa::OpenApi;

fn main() {
    // Generate the OpenAPI specification as JSON
    let openapi_json = ApiDoc::openapi().to_pretty_json().unwrap();
    
    // Write to file or serve via HTTP
    std::fs::write("openapi_spec.json", openapi_json).unwrap();
}
```

## Integration with Other Crates

The `openapi` crate integrates with several other parts of the Hyperswitch ecosystem:

1. **api_models**: Uses the API request and response models to generate schema documentation
2. **common_utils**: Incorporates utility types and enums in the schema documentation
3. **common_types**: Documents domain models used in the API
4. **router_env**: Uses environment configuration for server definitions

## Configuration Options

The crate supports the following feature flags:

- **v1**: Enables compatibility with v1 API models and documentation
- **v2**: Enables compatibility with v2 API models and documentation

These features allow the OpenAPI documentation to be generated for different API versions.

## Performance Considerations

The OpenAPI specification generation is typically done at build time or startup, not during regular operation:

- **Compile-time Preparation**: Most of the OpenAPI documentation is prepared at compile time using macros
- **One-time Generation**: The specification is usually generated once per API version
- **Serialization Efficiency**: The `utoipa` crate is designed for efficient serialization of the OpenAPI spec

## Conclusion

The `openapi` crate serves as the central documentation generator for the Hyperswitch API, providing comprehensive and up-to-date API specifications that can be used for API documentation, client generation, and testing.

## See Also

- [API Models Documentation](../api_models/overview.md)
- [Router Documentation](../router/overview.md)
- [OpenAPI Specification](https://spec.openapis.org/oas/latest.html)
