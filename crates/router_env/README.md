# router_env

Environment of payment router: logger, basic config, its environment awareness.

## Example

```rust
#[instrument]
pub fn sample() -> () {
    logger::log!(
        logger::Level::INFO,
        payment_id = 8565654,
        payment_attempt_id = 596456465,
        merchant_id = 954865,
        tag = ?logger::Tag::ApiIncomingRequest,
        category = ?logger::Category::Api,
        flow = "some_flow",
        session_id = "some_session",
    );
}
```

## Files Tree Layout

<!-- FIXME: fill missing -->

```text
├── src                        : source code
│   └── logger                 : logger
└── tests                      : unit and integration tests
    └── test_module            : unit and integration tests
```

<!--
command to generate the tree `tree -L 3 -d`
-->
