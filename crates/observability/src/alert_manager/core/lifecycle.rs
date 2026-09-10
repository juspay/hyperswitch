//! Per-request logic for alert lifecycle state and the announcements made about it.
//!
//! The alert manager touches its state exactly twice a run: it reads all of it, decides, and
//! writes all of it back. This layer is those two operations, plus the append that records what
//! was actually said.
//!
//! ## One handler, two channels
//!
//! `alerts_main` / `alerts_intermediate` and their `_xyne` twins are the same pair of tables once
//! per delivery channel. Diesel needs a struct per table, so the models are generated per channel —
//! but everything that decides anything is written once here and takes the channel as an argument.
//! The only code that branches on it is [`store`], whose functions are a two-arm match around one
//! query each.
//!
//! ## What the transaction does, in order
//!
//! A whole-state write is one transaction, and the order inside it is load bearing:
//!
//! 1. **Take the channel's advisory lock.** Without it two writers read the same watermark, both
//!    find their precondition satisfied, and the second overwrites the first.
//! 2. **Check the precondition.** The state must still be the state the caller read.
//! 3. **Check the announcements the rows reference exist**, so a foreign key cannot fail halfway
//!    through and so the failure names the id rather than the constraint.
//! 4. **Remove the rows the request no longer carries**, then **write the rows it does.** Both
//!    statements are against `alerts_intermediate` only.
//!
//! **A state write never touches `alerts_main`.** A state row references an announcement
//! `ON DELETE CASCADE`, so a replace that reached the announcement table would delete state rows
//! pointing at it — including ones this same request is writing. Announcements are appended by
//! [`record_announcement`] and removed by nothing.
//!
//! ## Whose clock, and what is checked here
//!
//! Every stored timestamp is [`common_utils::date_time::now`]. The columns have no `DEFAULT`, so
//! somebody has to choose, and a caller's clock being minutes out would make an episode's duration
//! wrong and then write that back. Column widths and the alert cap are checked before the query
//! runs, for the reason the dictionary checks its own: Postgres rejects the same values as an
//! opaque `22001` with a `500` attached, and one over-wide row would fail the whole batch.

use async_bb8_diesel::AsyncConnection;
use diesel_models::{
    observability::{
        alerts_intermediate::{slack as slack_state, xyne as xyne_state, AlertStateRow},
        alerts_main::{slack as slack_main, xyne as xyne_main, AnnouncementRow},
        raw_json::RawJson,
    },
    query::observability::alerts_intermediate::lock_lifecycle_state,
    DatabaseConnectionWithContext, StorageResult,
};
use error_stack::{report, ResultExt};
use time::PrimitiveDateTime;

use crate::{
    alert_manager::types::{
        lifecycle::{
            AlertStateEntry, AlertStateWrite, AnnouncementEntry, AnnouncementRequest,
            AnnouncementSaveResponse, Channel, LifecycleStateResponse, LifecycleStateSaveResponse,
            LifecycleStateWriteRequest,
        },
        ReadStatus, WriteStatus,
    },
    errors::{ObservabilityApiResult, ObservabilityError},
    logger,
    state::AppState,
};

/// `name`, `product`, `group_id` and `priority` are all `VARCHAR(64)`.
const NAME_MAX_BYTES: usize = 64;

/// `ts_slack` is `VARCHAR(255)`.
const TS_SLACK_MAX_BYTES: usize = 255;

/// The whole of one channel's lifecycle state.
///
/// A store with nothing in it answers [`ReadStatus::Absent`] and a `200`; a store that could not be
/// read is a `503`. The alert manager proceeds on the first and skips its run on the second, so the
/// two must never collapse into one another — see [`ObservabilityError::StorageUnavailable`].
pub async fn read_state(
    state: AppState,
    channel: Channel,
) -> ObservabilityApiResult<LifecycleStateResponse> {
    let connection = state.database_connection().await?;

    let rows = store::list(&connection, channel)
        .await
        // Not `InternalServerError`: from the caller's side a state table it cannot read is the
        // store being unusable, and answering `503` keeps this out of the 500s someone is paged
        // on while still being unmistakably not a `200`.
        .change_context(ObservabilityError::StorageUnavailable)
        .attach_printable("Failed to read the lifecycle state")?;

    Ok(LifecycleStateResponse {
        status: if rows.is_empty() {
            ReadStatus::Absent
        } else {
            ReadStatus::Found
        },
        // Folded from the rows rather than asked for separately: it is the same `MAX` the write
        // checks its precondition against, over the same rows, in one round trip instead of two.
        last_updated_at: rows.iter().filter_map(|row| row.last_updated_at).max(),
        alerts: rows.into_iter().map(AlertStateEntry::from).collect(),
    })
}

/// Replace the whole of one channel's lifecycle state, in one transaction.
pub async fn write_state(
    state: AppState,
    channel: Channel,
    request: LifecycleStateWriteRequest,
) -> ObservabilityApiResult<LifecycleStateSaveResponse> {
    let limit = state.conf.lifecycle.max_alerts;
    if request.alerts.len() > limit {
        // Logged with the counts, which the response deliberately does not carry: the caller knows
        // what it sent, and whoever is asked why the alert manager stopped writing does not.
        logger::warn!(
            alerts = request.alerts.len(),
            limit = limit,
            channel = channel.as_str(),
            "Lifecycle write rejected: over the configured alert cap"
        );
        Err(report!(ObservabilityError::StateTooLarge {
            alerts: request.alerts.len(),
            limit,
        }))?;
    }

    let now = stamp();
    let plan = WritePlan::build(request.alerts, now)?;
    let expected = request.expected_last_updated_at;
    let connection = state.database_connection().await?;

    // The connection handed to the closure is another handle to the one `connection` holds, so
    // every query issued through it below runs inside this transaction.
    let borrowed = &connection;
    let applied = borrowed
        .raw_connection()
        .transaction_async(move |_| async move {
            lock_lifecycle_state(borrowed, channel.lock_key()).await?;

            let found = store::watermark(borrowed, channel).await?;
            if found != expected {
                Err(WriteFailure::Stale { expected, found })?;
            }

            let existing =
                store::existing_announcements(borrowed, channel, plan.referenced.clone()).await?;
            if let Some(missing) = plan
                .referenced
                .iter()
                .find(|id| !existing.contains(id))
                .copied()
            {
                Err(WriteFailure::UnknownAnnouncement(missing))?;
            }

            // Remove first, then write. Both statements are against `alerts_intermediate`; the
            // announcement table is not touched, so nothing cascades.
            let removed = store::delete_absent(borrowed, channel, plan.keep).await?;
            let alerts = store::upsert_all(borrowed, channel, plan.rows).await?;

            Ok::<_, WriteFailure>(Applied { alerts, removed })
        })
        .await
        .map_err(|failure| failure.into_report(channel))?;

    Ok(LifecycleStateSaveResponse {
        status: WriteStatus::Saved,
        last_updated_at: now,
        alerts: applied.alerts,
        removed: applied.removed,
    })
}

/// Record one announcement against `alerts_main`.
///
/// An append, and deliberately not the write path above: this route adds a row and removes none.
/// Letting it inherit whole-replace semantics would delete announcements, and every state row
/// referencing a deleted announcement goes with it.
pub async fn record_announcement(
    state: AppState,
    channel: Channel,
    request: AnnouncementRequest,
) -> ObservabilityApiResult<AnnouncementSaveResponse> {
    fits(request.name.as_deref(), "name", NAME_MAX_BYTES)?;
    fits(request.product.as_deref(), "product", NAME_MAX_BYTES)?;
    fits(request.ts_slack.as_deref(), "ts_slack", TS_SLACK_MAX_BYTES)?;

    let now = stamp();
    let connection = state.database_connection().await?;

    let announcement = store::insert_announcement(
        &connection,
        channel,
        AnnouncementRow {
            // Minted here: there is no `gen_random_uuid()` on the column, and v7 sorts by the
            // instant it was made, so the announcement log reads in the order it happened.
            id: uuid::Uuid::now_v7(),
            name: request.name,
            product: request.product,
            dimensions: request.dimensions.map(RawJson::from),
            ts_slack: request.ts_slack,
            // The moment the announcement was recorded, by this service's clock. Durations are
            // measured against the state timestamps this same clock stamps.
            ts_alert: Some(now),
            duration: request.duration,
            sent: request.sent,
            critical: request.critical,
            rca_metadata: request.rca_metadata,
            metadata: request.metadata.map(RawJson::from),
            last_updated_at: Some(now),
        },
    )
    .await
    .change_context(ObservabilityError::InternalServerError)
    .attach_printable("Failed to record an announcement")?;

    Ok(AnnouncementSaveResponse {
        status: WriteStatus::Saved,
        announcement: AnnouncementEntry::from(announcement),
    })
}

/// How many rows a whole-state write left behind.
struct Applied {
    alerts: usize,
    removed: usize,
}

/// What a whole-state write does to the tables, worked out before the transaction opens.
///
/// Separated from the queries so the thing that has to be right — the order, and what is and is not
/// touched — can be read and tested without a database.
#[derive(Debug)]
struct WritePlan {
    /// The rows to write, stamped and with their ids settled.
    rows: Vec<AlertStateRow>,
    /// The ids the write keeps. Everything else is removed, so an empty plan clears the state.
    keep: Vec<uuid::Uuid>,
    /// The announcements the rows point at. Every one must exist before any row is written.
    referenced: Vec<uuid::Uuid>,
}

impl WritePlan {
    fn build(alerts: Vec<AlertStateWrite>, now: PrimitiveDateTime) -> ObservabilityApiResult<Self> {
        let mut rows = Vec::with_capacity(alerts.len());
        let mut keep = Vec::with_capacity(alerts.len());
        let mut referenced = Vec::new();

        for alert in alerts {
            fits(alert.name.as_deref(), "name", NAME_MAX_BYTES)?;
            fits(alert.product.as_deref(), "product", NAME_MAX_BYTES)?;
            fits(alert.group_id.as_deref(), "group_id", NAME_MAX_BYTES)?;
            fits(alert.priority.as_deref(), "priority", NAME_MAX_BYTES)?;
            fits(alert.ts_slack.as_deref(), "ts_slack", TS_SLACK_MAX_BYTES)?;

            // Minted when the caller has none, which is every alert it has just detected.
            let id_intermediate = alert.id_intermediate.unwrap_or_else(uuid::Uuid::now_v7);

            if keep.contains(&id_intermediate) {
                // Postgres refuses to touch the same row twice in one `ON CONFLICT DO UPDATE`, so
                // this would otherwise fail the whole batch with a message about the statement
                // rather than about the request.
                Err(
                    report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                        "The lifecycle write carries id_intermediate {id_intermediate} twice"
                    )),
                )?;
            }

            keep.push(id_intermediate);
            if let Some(announcement) = alert.announcement_id {
                if !referenced.contains(&announcement) {
                    referenced.push(announcement);
                }
            }

            rows.push(AlertStateRow {
                id_intermediate,
                id: alert.announcement_id,
                name: alert.name,
                product: alert.product,
                dimensions: alert.dimensions,
                ts_slack: alert.ts_slack,
                ts_alert: alert.ts_alert,
                latest_ts_alert: alert.latest_ts_alert,
                max_duration: alert.max_duration,
                other_metrics: alert.other_metrics,
                metadata: alert.metadata,
                metadata_alert_details: alert.metadata_alert_details,
                rca_metadata: alert.rca_metadata,
                group_id: alert.group_id,
                priority: alert.priority,
                // The server's clock, on every row, every time. It is also the precondition the
                // next write is checked against, so it has to come from one place.
                last_updated_at: Some(now),
                recovered_ts: alert.recovered_ts,
            });
        }

        Ok(Self {
            rows,
            keep,
            referenced,
        })
    }
}

/// Why a whole-state write did not apply.
///
/// A type of its own because the transaction helper needs an error it can build from a diesel
/// failure, and because two of these are outcomes the caller has to tell apart rather than
/// failures: a stale precondition and a dangling reference are both well-formed requests.
enum WriteFailure {
    /// The state moved after the caller read it.
    Stale {
        expected: Option<PrimitiveDateTime>,
        found: Option<PrimitiveDateTime>,
    },
    /// A row referenced an announcement that does not exist.
    UnknownAnnouncement(uuid::Uuid),
    /// A query failed.
    Storage(error_stack::Report<diesel_models::errors::DatabaseError>),
    /// The transaction itself failed to begin, commit or roll back.
    Transaction(diesel::result::Error),
}

impl WriteFailure {
    fn into_report(self, channel: Channel) -> error_stack::Report<ObservabilityError> {
        match self {
            // The two timestamps go to the log and not to the response: they say when the winning
            // run wrote, and the loser can do nothing with that but read the state again.
            Self::Stale { expected, found } => {
                logger::warn!(
                    channel = channel.as_str(),
                    expected = ?expected,
                    found = ?found,
                    "Lifecycle write rejected: the state changed after it was read"
                );
                report!(ObservabilityError::StateChanged)
            }
            Self::UnknownAnnouncement(id) => {
                report!(ObservabilityError::UnknownAnnouncement { id: id.to_string() })
            }
            Self::Storage(error) => error
                .change_context(ObservabilityError::StorageUnavailable)
                .attach_printable("Failed to write the lifecycle state"),
            Self::Transaction(error) => report!(ObservabilityError::StorageUnavailable)
                .attach_printable(format!("The lifecycle write transaction failed: {error}")),
        }
    }
}

impl From<diesel::result::Error> for WriteFailure {
    fn from(error: diesel::result::Error) -> Self {
        Self::Transaction(error)
    }
}

impl From<error_stack::Report<diesel_models::errors::DatabaseError>> for WriteFailure {
    fn from(error: error_stack::Report<diesel_models::errors::DatabaseError>) -> Self {
        Self::Storage(error)
    }
}

/// The instant this service stamps a row with, to the precision the wire carries.
///
/// **Truncated to the millisecond on purpose.** `last_updated_at` is handed back to the caller as
/// the precondition for its next write, and this service's clock has more precision than the
/// ISO 8601 format the API is written in — so a stamp kept at full precision could not be echoed
/// back exactly, and every whole-state write after the first would be refused as stale. Truncating
/// what is stored, rather than comparing loosely, keeps the precondition an equality.
fn stamp() -> PrimitiveDateTime {
    let now = common_utils::date_time::now();

    now.replace_millisecond(now.millisecond()).unwrap_or(now)
}

/// Check a value against its column's width.
///
/// An absent value is fine — no column here carries `NOT NULL`, so a `null` is a stored fact.
fn fits(value: Option<&str>, field: &'static str, max_bytes: usize) -> ObservabilityApiResult<()> {
    if let Some(value) = value {
        if value.len() > max_bytes {
            Err(
                report!(ObservabilityError::InvalidRequest).attach_printable(format!(
                    "The lifecycle {field} is {} bytes, over the {max_bytes} the column holds",
                    value.len()
                )),
            )?;
        }
    }

    Ok(())
}

/// The only code in this module that knows there are two channels.
///
/// Each function is one query behind a two-arm match. The models are generated per table because
/// diesel needs a struct per table; they take and return the channel-agnostic row, so nothing
/// above here is written twice.
mod store {
    use super::{
        slack_main, slack_state, xyne_main, xyne_state, AlertStateRow, AnnouncementRow, Channel,
        DatabaseConnectionWithContext, PrimitiveDateTime, StorageResult,
    };

    pub(super) async fn list(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
    ) -> StorageResult<Vec<AlertStateRow>> {
        match channel {
            Channel::Slack => slack_state::AlertState::list(conn).await,
            Channel::Xyne => xyne_state::AlertState::list(conn).await,
        }
    }

    pub(super) async fn watermark(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
    ) -> StorageResult<Option<PrimitiveDateTime>> {
        match channel {
            Channel::Slack => slack_state::AlertState::latest_last_updated_at(conn).await,
            Channel::Xyne => xyne_state::AlertState::latest_last_updated_at(conn).await,
        }
    }

    pub(super) async fn delete_absent(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        keep: Vec<uuid::Uuid>,
    ) -> StorageResult<usize> {
        match channel {
            Channel::Slack => slack_state::AlertState::delete_absent(conn, keep).await,
            Channel::Xyne => xyne_state::AlertState::delete_absent(conn, keep).await,
        }
    }

    pub(super) async fn upsert_all(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        rows: Vec<AlertStateRow>,
    ) -> StorageResult<usize> {
        match channel {
            Channel::Slack => slack_state::AlertState::upsert_all(conn, rows).await,
            Channel::Xyne => xyne_state::AlertState::upsert_all(conn, rows).await,
        }
    }

    pub(super) async fn existing_announcements(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        ids: Vec<uuid::Uuid>,
    ) -> StorageResult<Vec<uuid::Uuid>> {
        match channel {
            Channel::Slack => slack_main::Announcement::existing_ids(conn, ids).await,
            Channel::Xyne => xyne_main::Announcement::existing_ids(conn, ids).await,
        }
    }

    pub(super) async fn insert_announcement(
        conn: &DatabaseConnectionWithContext<'_>,
        channel: Channel,
        row: AnnouncementRow,
    ) -> StorageResult<AnnouncementRow> {
        match channel {
            Channel::Slack => slack_main::Announcement::insert(conn, row).await,
            Channel::Xyne => xyne_main::Announcement::insert(conn, row).await,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
mod tests {
    use super::*;

    fn alert(id: Option<uuid::Uuid>, announcement: Option<uuid::Uuid>) -> AlertStateWrite {
        AlertStateWrite {
            id_intermediate: id,
            announcement_id: announcement,
            name: Some("sr_drop".to_owned()),
            product: Some("payments".to_owned()),
            dimensions: None,
            ts_slack: None,
            ts_alert: None,
            latest_ts_alert: None,
            max_duration: None,
            other_metrics: None,
            metadata: None,
            metadata_alert_details: None,
            rca_metadata: None,
            group_id: Some("sr_drop|merchant_1234".to_owned()),
            priority: Some("SEV2".to_owned()),
            recovered_ts: None,
        }
    }

    fn now() -> PrimitiveDateTime {
        common_utils::date_time::now()
    }

    /// The whole point of the cascade: an announcement must exist before a state row points at it,
    /// so the plan collects the references for the check that runs before anything is written.
    #[test]
    fn a_plan_collects_the_announcements_its_rows_reference() {
        let first = uuid::Uuid::now_v7();
        let second = uuid::Uuid::now_v7();

        let plan = WritePlan::build(
            vec![
                alert(None, Some(first)),
                alert(None, Some(second)),
                // Two rows under one announcement: an episode announced once and still firing.
                alert(None, Some(first)),
                alert(None, None),
            ],
            now(),
        )
        .unwrap();

        assert_eq!(plan.referenced, vec![first, second]);
        assert_eq!(plan.rows.len(), 4);
        assert_eq!(plan.keep.len(), 4);
    }

    /// A whole-state write is a replacement, so what it does not carry goes. An empty request is
    /// the recovery of everything, not a no-op.
    #[test]
    fn a_write_carrying_nothing_keeps_nothing() {
        let plan = WritePlan::build(Vec::new(), now()).unwrap();

        assert!(plan.keep.is_empty());
        assert!(plan.rows.is_empty());
        assert!(plan.referenced.is_empty());
    }

    /// Every row is stamped by this service, and with the same instant, because that stamp is the
    /// precondition the next write is checked against.
    #[test]
    fn every_row_is_stamped_with_the_servers_clock() {
        let at = now();
        let plan = WritePlan::build(vec![alert(None, None), alert(None, None)], at).unwrap();

        for row in &plan.rows {
            assert_eq!(row.last_updated_at, Some(at));
        }
    }

    /// A caller that has just detected an alert has no id to send, and the column has no default.
    #[test]
    fn a_row_without_an_id_is_given_one() {
        let plan = WritePlan::build(vec![alert(None, None)], now()).unwrap();

        assert_ne!(plan.rows[0].id_intermediate, uuid::Uuid::nil());
        assert_eq!(plan.keep[0], plan.rows[0].id_intermediate);
    }

    /// An id the caller echoed back updates that row rather than replacing it, which is what keeps
    /// the episode's start and its thread.
    #[test]
    fn a_row_with_an_id_keeps_it() {
        let id = uuid::Uuid::now_v7();
        let plan = WritePlan::build(vec![alert(Some(id), None)], now()).unwrap();

        assert_eq!(plan.rows[0].id_intermediate, id);
        assert_eq!(plan.keep, vec![id]);
    }

    /// Postgres refuses to touch the same row twice in one upsert, and the message it gives names
    /// the statement rather than the request.
    #[test]
    fn the_same_row_twice_in_one_write_is_rejected() {
        let id = uuid::Uuid::now_v7();
        let error = WritePlan::build(vec![alert(Some(id), None), alert(Some(id), None)], now())
            .unwrap_err();

        assert!(matches!(
            error.current_context(),
            ObservabilityError::InvalidRequest
        ));
    }

    /// Postgres would reject it too, as a `22001` that arrives as an opaque failure with a `500`
    /// attached — and one over-wide row would take the whole batch with it.
    #[test]
    fn a_value_wider_than_its_column_is_rejected_before_the_query_runs() {
        let mut wide = alert(None, None);
        wide.group_id = Some("g".repeat(NAME_MAX_BYTES + 1));

        assert!(WritePlan::build(vec![wide], now()).is_err());

        let mut long_thread = alert(None, None);
        long_thread.ts_slack = Some("t".repeat(TS_SLACK_MAX_BYTES + 1));

        assert!(WritePlan::build(vec![long_thread], now()).is_err());
    }

    /// The precondition is an equality, so the value this service stores has to be one the wire
    /// format can carry exactly. Without this every whole-state write after the first would be
    /// refused as stale, and nothing would ever be written again.
    #[test]
    fn a_stamp_survives_the_wire_format_unchanged() {
        #[derive(serde::Deserialize, serde::Serialize)]
        struct Carried(#[serde(with = "common_utils::custom_serde::iso8601")] PrimitiveDateTime);

        for _ in 0..1_000 {
            let stamped = stamp();
            let wire = serde_json::to_string(&Carried(stamped)).unwrap();
            let returned: Carried = serde_json::from_str(&wire).unwrap();

            assert_eq!(returned.0, stamped, "a stamp did not survive {wire}");
        }
    }

    /// No column here is `NOT NULL`, so an absent value is a stored fact and not a short one.
    #[test]
    fn an_absent_value_is_not_measured_against_its_column() {
        let mut bare = alert(None, None);
        bare.name = None;
        bare.group_id = None;

        assert!(WritePlan::build(vec![bare], now()).is_ok());
    }
}
