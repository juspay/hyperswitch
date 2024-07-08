# router_env

Environment of payment router: logger, basic config, its environment awareness.

## Example

```rust
use router_env::logger;
use tracing::{self, instrument};

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
