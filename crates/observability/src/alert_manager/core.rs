pub mod config;
pub mod dictionary;
pub mod instances;
pub mod lifecycle;
pub mod notifications;

use time::PrimitiveDateTime;

/// The instant this service stamps a row with, to the precision the wire carries.
pub(super) fn stamp() -> PrimitiveDateTime {
    let now = common_utils::date_time::now();

    now.replace_millisecond(now.millisecond()).unwrap_or(now)
}
