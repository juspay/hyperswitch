---
name: hyperswitch-add-connector
description: >-
  Add a new payment processor connector to Hyperswitch: scaffold it with
  scripts/add_connector.sh, generate request/response types from the
  processor's OpenAPI spec, implement transformers and the ConnectorCommon /
  ConnectorIntegration traits, protect PII, and test it. Use when
  contributing or modifying a connector in crates/hyperswitch_connectors.
---

# Adding a Hyperswitch connector

References: `add_connector.md`, `add_connector_updated.md`,
`connector-template/`, `crates/hyperswitch_connectors/src/connectors/`.

## Scaffold

```bash
sh scripts/add_connector.sh <connectorname> <ConnectorBaseUrl>   # single-word name
```

Creates:

```
crates/hyperswitch_connectors/src/connectors/<name>.rs              # trait impls
crates/hyperswitch_connectors/src/connectors/<name>/transformers.rs # request/response types + conversions
crates/router/tests/connectors/<name>.rs                            # test boilerplate
```

Test failures right after scaffolding are expected.

## Types from the processor's spec

```bash
curl -o <name>-openapi.json <schema-url>      # e.g. /swagger/v1/swagger.json, /openapi.json
export CONNECTOR_NAME=<name> SCHEMA_PATH=/abs/path/<name>-openapi.json
# run openapi-generator as described in add_connector.md -> connectors/<name>/temp.rs
```

Copy only the structs you need from `temp.rs` into `transformers.rs`.

## Implement

1. **transformers.rs**: `TryFrom` conversions from router data to the
   connector request, and from the connector response to router data.
   Map the processor's statuses to Hyperswitch attempt/refund statuses
   carefully; wrong mappings cause false successes.
2. **`<name>.rs`**: `ConnectorCommon` (id, base URL, auth header, error
   response parsing) and `ConnectorIntegration` per flow (authorize,
   capture, psync, void, refund, rsync, webhooks).
3. Choose the pattern from the guide: direct authorization or
   tokenization-first.
4. PII: wrap card data, emails and secrets in `Secret` / `common_utils::pii`
   types; never log them.
5. Control Center: update `ConnectorTypes.res`, `ConnectorUtils.res` and
   icons in the control-center repo if the connector should appear there.

## Test

```bash
cargo run                                            # router on :8080
curl --head http://localhost:8080/health
cargo test --package router --test connectors -- <name> --test-threads=1
cargo +nightly fmt --all && cargo clippy
```

Use the processor's sandbox credentials, never production keys. Follow
the PR template checklist (fmt, clippy, tests) when opening the PR.
