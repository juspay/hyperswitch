//! Request correlation context shared by infrastructure clients.
use std::sync;

use crate::read_ledger;

/// Minimal request correlation context for infrastructure calls.
///
/// This intentionally avoids the name `Context` to prevent confusion with
/// `error_stack::Context`, and carries the request ID only.
pub trait RequestContext: Send + Sync {
    /// Return the request ID associated with the current execution, if any.
    fn request_id(&self) -> Option<&str>;

    /// The ledger of records this API session has already read, if it has one.
    /// `None` by default: background callers (drainer, scheduler, consumer) are
    /// not sessions, so there is nothing for a read to repeat *within*.
    fn read_ledger(&self) -> Option<&sync::Arc<read_ledger::ReadLedger>> {
        None
    }
}
