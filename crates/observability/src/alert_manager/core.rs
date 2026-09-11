pub mod config;
pub mod dictionary;
pub mod instances;
pub mod lifecycle;
pub mod notifications;

use time::PrimitiveDateTime;

pub(super) fn stamp() -> PrimitiveDateTime {
    let now = common_utils::date_time::now();

    now.replace_millisecond(now.millisecond()).unwrap_or(now)
}
