# Toggle for process_tracker task creation

Date: 2026-09-08
Status: implemented

## Goal

Add a configuration toggle that stops ALL inserts into the `process_tracker` table.
The toggle mirrors the existing `webhooks.outgoing_enabled` pattern: when the toggle is off,
the application continues to serve API traffic normally,
but no new rows enter the `process_tracker` table.

The consumer loop already has its own toggle: `scheduler.consumer.disabled`.
This document covers inserts only.
Producer and consumer loop behavior does not change.

## Decisions (from the grilling round)

1. Gate placement: a wrapper function at the write site, not inside the storage layer.
   Reason: the skip decision is a routing concern, not a storage concern.
2. Scope: block every insert, including inserts executed inside the consumer pod by workflow code
   (dispute-list reschedule, refund re-add after execute, revenue-recovery task chain).
3. Config key: `[scheduler] task_creation_enabled = true` (positive polarity, accepted).
4. Flag threading: leaf helpers receive `&SchedulerSettings` (not a plain bool).
5. Metrics: the wrapper owns both counters.
   It increments `TASKS_ADDED_COUNT` only on a real insert,
   and increments a new skip counter plus one tagged log line per skipped task.
   Existing per-helper `flow` label values are preserved by passing them into the wrapper.
6. Accepted consequence: objects created while the toggle is off never get scheduler tasks,
   and re-enabling the toggle does not create them retroactively.
   Manual retrieve endpoints (payments PSync, refund retrieve) are the recovery path.
7. Cleanup in scope: delete the dead `consumer::create_task` helper
   and the commented-out `add_delete_tokenized_data_task` lines in `vault.rs`.
8. Destination: upstream-quality change (user is on the hyperswitch team).

## Config

| Place | Change |
|---|---|
| `crates/scheduler/src/configs/settings.rs` | Add `pub task_creation_enabled: bool` to `SchedulerSettings` |
| `crates/scheduler/src/configs/defaults.rs` | Default `true` (no behavior change for existing deployments) |
| `config/development.toml`, `config/config.example.toml`, `config/docker_compose.toml` | New key under `[scheduler]`, with a doc comment describing semantics |
| `config/deployments/scheduler/consumer.toml` | Same key. Required because the consumer pod executes workflows that insert follow-up rows using the consumer's own config |
| `config/deployments/scheduler/producer.toml` | No change. The producer never inserts |

Env override follows existing conventions: `ROUTER__SCHEDULER__TASK_CREATION_ENABLED=false`.

`Settings.scheduler` is `Option<SchedulerSettings>`.
A new accessor `Settings::scheduler_settings()` returns `self.scheduler.clone().unwrap_or_default()`,
so helpers receive a plain `&SchedulerSettings` with the flag defaulted to `true`.

## The wrapper

New module: `crates/router/src/db/process_tracker.rs`, registered in `crates/router/src/db.rs`.

Function: `insert_process_if_task_creation_enabled`.

Parameters:
- `db: &dyn StorageInterface`
- `entry: storage::ProcessTrackerNew`
- `scheduler_settings: &SchedulerSettings`
- `flow_attributes: Option<&[opentelemetry::KeyValue]>`
  (existing per-helper `flow` attributes where a helper historically counted,
  `None` where it did not; `metric_attributes!` returns a slice reference,
  so the wrapper cannot own the attributes)

Behavior:
- Flag on: delegate to `db.insert_process(entry)`,
  then increment `TASKS_ADDED_COUNT` with the given attributes (if any).
- Flag off: emit one tagged `logger::info!` (task id, name, runner),
  increment `PROCESS_TRACKER_TASK_CREATION_SKIPPED_COUNT`
  (added in `crates/router/src/routes/metrics.rs`),
  and return a synthesized `ProcessTracker` built via a new
  `impl From<ProcessTrackerNew> for ProcessTracker`
  in `crates/diesel_models/src/process_tracker.rs`.
  The two structs are field-identical, so the synthesized row is complete.
  `TASKS_ADDED_COUNT` and `TASKS_ADDITION_FAILURES_COUNT` are not touched on the skip path.
  No duplicate-key error can occur on the skip path;
  the `DuplicateRefundRequest` dedupe behavior is silently absent while disabled (accepted).

Per-site metric handling:
- Helpers that incremented `TASKS_ADDED_COUNT` at insertion time
  pass their attributes through as `Some(...)`.
- `add_domain_task_to_pt` and the cancel-post-capture operation incremented at decision time;
  that increment moves into the wrapper call as `Some(...)` with the same attribute value.
  A subsequent insert failure now drops the count (accepted drift).
- Dispute helpers that counted before attempting the insert
  pass those attributes into the wrapper instead.
- The outgoing-webhook retry path keeps its `TASK_ADDITION_FAILURES_COUNT` increment:
  it is attached with `inspect_err` on the wrapper result.
- Helpers and sites that never counted now count when enabled (accepted).

## Migration of insert sites

### Leaf helpers (gain a `&SchedulerSettings` param, gate via the wrapper)

| Helper | File (line) |
|---|---|
| `add_process_sync_task` | `crates/router/src/core/payments.rs` (11116) |
| `add_process_post_capture_void_sync_task` | `crates/router/src/core/payments.rs` (11155) |
| `add_refund_sync_task`, `add_refund_execute_task` (v1) | `crates/router/src/core/refunds.rs` (2587, 2651) |
| `add_refund_execute_task`, `add_refund_sync_task` (v2) | `crates/router/src/core/refunds_v2.rs` (1502, 1541) |
| `add_process_dispute_task_to_pt`, `add_dispute_list_task_to_pt` | `crates/router/src/core/disputes.rs` (993, 1044) |
| `add_external_account_addition_task` | `crates/router/src/core/payouts.rs` (3692) |
| `add_payment_method_status_update_task` | `crates/router/src/core/payment_methods.rs` (641) |
| `add_payment_method_modular_forward_compat_task` | `crates/router/src/core/payment_methods.rs` (701) |
| `add_network_tokenization_task` (v1 + v2) | `crates/router/src/core/payment_methods.rs` (755, 802) |
| `add_payment_method_modular_backward_compat_task` | `crates/router/src/core/payment_methods.rs` (891) |
| `add_delete_tokenized_data_task` | `crates/router/src/core/payment_methods/vault.rs` (3222) |
| `add_api_key_expiry_task` | `crates/router/src/core/api_keys.rs` (197) |
| `add_outgoing_webhook_retry_task_to_process_tracker` | `crates/router/src/core/webhooks/outgoing.rs` (1024) |
| `insert_psync_pcr_task_to_pt` | `crates/router/src/core/revenue_recovery.rs` (490) |

Metric-increment lines previously inside these helpers are folded into the wrapper calls
(see the wrapper section for the exact rules per site).

### State-carrying sites (read conf directly, gate via the wrapper)

| Site | File (line) |
|---|---|
| `insert_notification_task` (offer engine) | `crates/router/src/core/offer_engine/notify.rs` (128) |
| batch blocklist upload | `crates/router/src/core/blocklist/batch.rs` (438) |
| `upsert_calculate_pcr_task` | `crates/router/src/core/revenue_recovery.rs` (68) |
| `insert_execute_pcr_task_to_pt` | `crates/router/src/core/revenue_recovery.rs` (1042) |
| `reopen_calculate_workflow_on_payment_failure` | `crates/router/src/core/revenue_recovery/types.rs` (1364) |
| `add_payout_sync_task_to_process_tracker` | `crates/router/src/workflows/payout_sync.rs` (145) |

Note: three sites previously called `db.as_scheduler().insert_process` (`revenue_recovery.rs` 177, 533;
`types.rs` 1538).
The wrapper takes `&dyn StorageInterface`, not `&dyn SchedulerInterface`;
those sites pass the plain store reference instead.

### The subscriptions exception (inline gate, not the wrapper)

The `subscriptions` crate does not depend on the `router` crate, so it cannot call the wrapper.
Changes instead:
- `crates/subscriptions/src/state.rs`: `SubscriptionConfig` gains a
  `scheduler: scheduler::SchedulerSettings` field.
- `crates/router/src/types/domain/types.rs`: the `From<SessionState> for SubscriptionState`
  conversion populates it from `state.conf.scheduler_settings()`.
- `crates/subscriptions/src/workflows/invoice_sync.rs` (`create_invoice_sync_job`):
  gates the `state.store.insert_process` call inline against
  `state.conf.scheduler.task_creation_enabled`
  and returns `Ok(())` early with its own `logger::info!` skip line when disabled.
  This site never counted router metrics, so no metric moves with the gate.

### Callers that pass the flag down (no gate inside)

`payments/helpers.rs` (`add_domain_task_to_pt`), `payment_cancel_post_capture.rs` (324),
`schedule_refund_execution` (v1 + v2), `disputes.rs` (908, 1152),
`workflows/dispute_list.rs` (231), `payouts.rs` (1479, 2785),
`cards.rs` (1450, 2422), `core/migration/payment_methods.rs` (579),
`vault.rs` (1841), `api_keys.rs` (174, 356),
outgoing-webhook caller of `add_outgoing_webhook_retry_task_to_process_tracker`,
`webhooks/recovery_incoming.rs` (301), `revenue_recovery.rs` (413, 1434), `types.rs` (836),
`subscriptions` `core.rs` (276, 415), `webhooks.rs` (130), `invoice_handler.rs` (286).

### Trait ripple

No trait changes needed.
The `PaymentMethodCreate` controller method `add_payment_method_status_update_task`
keeps its signature; implementations own `self.state`
and pass `&self.state.conf.scheduler_settings()` at the inner helper call.

### Deletions

- `crates/scheduler/src/consumer.rs` (289-295): `create_task`, zero callers.
- `crates/router/src/core/payment_methods/vault.rs` (1900-1902): commented-out twin
  of the live call at line 1841 (the payouts locker path was deliberately never enabled).

## Process behavior while disabled

- Inserts: all blocked (decision 2).
- Updates (`update_process`, `ProcessTrackerUpdate::*`, status transitions, limbo reinit): NOT blocked.
  Only inserts change.
- Existing rows: consumers with `consumer.disabled = false` keep processing already-pending rows.
  To freeze a cluster fully, also set `scheduler.consumer.disabled = true` and scale the producer to 0.
- `TASKS_ADDED_COUNT` shows the true insert rate (it drops to 0 while disabled);
  the skip counter shows what would have been created.

## Non-goals

- No change to producer/consumer loop behavior or to `consumer.disabled`.
- No retroactive reconciliation sweep for the disabled window.
- No gate on updates or on reads from `process_tracker`.

## Verification (justfile recipes only; do not call cargo directly)

1. `just fmt`
2. `just clippy` — passes (v1 features, `--all-targets` compiles the wrapper tests).
3. `just clippy_v2` — passes with `-D warnings` (v2 features).
4. Two unit tests in `crates/router/src/db/process_tracker.rs`:
   - insert-when-enabled: row present in MockDb, real insert path.
   - skip-when-disabled: MockDb stays empty, synthesized row returned, skip log/metric path.
   This checkout has no `just test` recipe; the tests are compiled by the `--all-targets`
   clippy runs and can be executed with `cargo test -p router db::process_tracker`.
5. Smoke run (manual): run the router with `task_creation_enabled = false`,
   issue `POST /payments`, confirm zero new rows in `process_tracker` and the tagged skip log line.

Cypress/postman and migrations: no change (default value preserves current behavior).
