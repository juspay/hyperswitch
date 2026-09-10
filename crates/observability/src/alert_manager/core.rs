pub mod config;
pub mod dictionary;
pub mod instances;
pub mod lifecycle;
pub mod notifications;

use time::PrimitiveDateTime;

/// The instant this service stamps a row with, to the precision the wire carries.
///
/// **Truncated to the millisecond on purpose.** [`lifecycle`] hands `last_updated_at` back to the
/// caller as the precondition for its next write, and this service's clock has more precision than
/// the ISO 8601 format the API is written in — so a stamp kept at full precision could not be
/// echoed back exactly, and every whole-state write after the first would be refused as stale.
/// Truncating what is stored, rather than comparing loosely, keeps the precondition an equality.
///
/// Shared by every resource that stamps a row rather than owned by the one that needs the
/// equality, so that two rows written by the same request — an announcement and the instances
/// hanging off it — carry timestamps of the same precision.
pub(super) fn stamp() -> PrimitiveDateTime {
    let now = common_utils::date_time::now();

    now.replace_millisecond(now.millisecond()).unwrap_or(now)
}
