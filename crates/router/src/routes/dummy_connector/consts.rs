pub const ATTEMPT_ID_PREFIX: &str = "dummy_attempt";
#[cfg(all(feature = "dummy_connector", feature = "v1"))]
pub const REFUND_ID_PREFIX: &str = "dummy_ref";
pub const THREE_DS_CSS: &str = include_str!("threeds_page.css");
