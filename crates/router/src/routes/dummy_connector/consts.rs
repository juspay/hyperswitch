pub const ATTEMPT_ID_PREFIX: &str = "dummy_attempt";
#[cfg_attr(feature = "v2", allow(dead_code))] // This is not used in v2
pub const REFUND_ID_PREFIX: &str = "dummy_ref";
#[cfg_attr(feature = "v2", allow(dead_code))] // This is not used in v2
pub const THREE_DS_CSS: &str = include_str!("threeds_page.css");
pub const DUMMY_CONNECTOR_UPI_FAILURE_VPA_ID: &str = "failure@upi";
pub const DUMMY_CONNECTOR_UPI_SUCCESS_VPA_ID: &str = "success@upi";
