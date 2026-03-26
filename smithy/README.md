# Smithy SDK Generation

This directory contains Smithy model definitions and configuration for generating client SDKs in multiple languages.

## Overview

The workflow consists of two phases:
1. **Rust → Smithy IDL**: Rust structs with `#[derive(SmithyModel)]` are converted to Smithy IDL
2. **Smithy IDL → SDKs**: Smithy CLI generates client SDKs

## Prerequisites

The smithy CLI is available via nix:

```bash
nix develop
smithy --version  # Should show 1.55.0
```

## Quick Start

### Generate Smithy IDL from Rust

```bash
# Using cargo
cargo run --package smithy-generator

# Or using just
just smithy-idl
```

This generates:
- `smithy/models/com_hyperswitch_smithy_types.smithy` - All types from Rust structs
- `smithy/models/com_hyperswitch_default.smithy` - Default namespace types

### Validate Smithy Models

```bash
# Using smithy CLI directly
smithy validate smithy/models/

# Or using just
just smithy-validate
```

**Validation Status**: ✅ SUCCESS (3257 shapes, 687 warnings about enum naming conventions)

### Generate Client SDKs

```bash
# Generate all SDKs
cd smithy && smithy build

# Or using just
just smithy-build
```

### Generate TypeScript SDK Only

```bash
just smithy-ts
```

**TypeScript SDK Status**: ✅ Working - Generates full client with commands for Payments, Refunds, Customers, Mandates

**Location**: `smithy/sdk/typescript/typescript-codegen/`

## Directory Structure

```
smithy/
├── models/
│   ├── com_hyperswitch.smithy          # Service definition (manually written)
│   ├── com_hyperswitch_smithy_types.smithy  # Generated from Rust
│   └── com_hyperswitch_default.smithy  # Generated from Rust
├── smithy-build.json                    # Build configuration
├── sdk/                                 # Generated SDKs (gitignored)
│   └── typescript/
│       └── typescript-codegen/
│           ├── src/
│           │   ├── commands/            # API operation commands
│           │   ├── models/              # Type definitions
│           │   └── HyperswitchClient.ts # Main client
│           └── package.json
└── README.md                           # This file
```

## Build Configuration

The `smithy-build.json` configures:

- **source**: OpenAPI spec generation (requires protocol trait - currently disabled)
- **typescript**: TypeScript client SDK ✅ Working

## Output

Generated SDKs are written to the `sdk/` directory:

```
sdk/
├── typescript/typescript-codegen/    # TypeScript client SDK
└── source/                           # Smithy model artifacts
```

**Note**: The `sdk/` directory is gitignored to prevent committing generated code.

## Current Status

### ✅ Working
- Smithy CLI via nix develop
- Model validation (SUCCESS)
- TypeScript client SDK generation with full API coverage

### ⚠️ Warnings (Non-blocking)
- 687 enum naming convention warnings (snake_case instead of PascalCase)
- Missing `@readonly` traits on GET operations
- Missing protocol trait for OpenAPI generation

### 🔧 To Do
- Python SDK (smithy-python codegen)
- Java SDK (smithy-java codegen)
- Rust SDK (smithy-rs codegen)
- Go SDK (waiting for smithy-go to mature)

## SDK Usage Example

```typescript
import { HyperswitchClient, PaymentsCreateCommand } from "@juspay/hyperswitch-client";

const client = new HyperswitchClient({
  endpoint: "https://api.hyperswitch.io",
  // ... auth config
});

const response = await client.send(new PaymentsCreateCommand({
  payload: {
    amount: 1000,
    currency: "USD",
    // ...
  }
}));
```

## References

- [AWS Smithy Documentation](https://smithy.io/2.0/index.html)
- [Smithy TypeScript Codegen](https://github.com/smithy-lang/smithy-typescript)
- [Smithy Python Codegen](https://github.com/smithy-lang/smithy-python)
- [Smithy Rust Codegen](https://github.com/smithy-lang/smithy-rs)
- [Superposition Smithy Reference](https://github.com/juspay/superposition/tree/main/smithy)
