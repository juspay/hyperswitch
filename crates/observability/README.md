# observability

The observability plane for Hyperswitch.

`observability` is the home for alert *delivery* and for the alert manager's own *state*. Deciding
what is alert-worthy — thresholds, detectors, suppression — is **not** done here; alerts arrive
already decided, and storing a threshold is not applying it.

Two concerns live here today. The [`notifier`](src/domain/notifier.rs) receives alert data over a
webhook and delivers it to a channel. The configuration routes own the rows the alert manager reads:
what an alert is and whether it runs, together with what it used to write into the application's
ClickHouse — the mappers and the notification bell's read watermark.

## Shape

The crate ships two ways, on the `drainer` model:

- **Standalone** — `cargo run -p observability`, its own `actix` `HttpServer`, released
  independently.
- **Embedded** — a library exposing an `actix` `Scope` the router can mount in-process.

Only the standalone path is wired up today. Mounting in the router is deliberately deferred; the
crate exposes `Scope` factories rather than raw handlers so both paths share one route definition
when that lands.

### Why the deployed name differs

The standalone deployment is **`hyperswitch-observability-plane`** — that is the name in ECR, in
the Helm chart and in ArgoCD. The crate is `observability`, without the suffix, and the difference
is deliberate rather than an oversight.

"Plane" is a deployment-topology word: a tier deployed and scaled apart from the data path. That
is true of the standalone binary and false of the embedded library, which runs inside the router's
process where there is no separate plane at all. The suffix therefore belongs to the artifact that
is one, and not to the crate that is both.

The binary keeps the crate's name, so the Dockerfile takes `BINARY=observability`.

## Versioning

`observability` has **no `v1`/`v2` API semantics of its own** and uses no version-flavoured type.
It carries the two features anyway, because the crates it reaches through — `hyperswitch_interfaces`
for secrets management, `diesel_models` for its schema — do not build unless one is selected. The
default is `v1`, matching the router, so nobody has to think about a choice this crate does not
make. Keep it that way: a version-flavoured type in a signature here would drag the matrix in for
real.

## Configuration

Reads `config/observability.toml` by default; override with `-f <path>`. Every value can be
overridden by an environment variable prefixed `OBSERVABILITY__`, with `__` separating levels — so
`OBSERVABILITY__AUTH__INTERNAL_API_KEY` sets `auth.internal_api_key`.

Configuration is validated at boot and startup fails loudly on a missing internal API key, rather
than on the first request.

## Authentication

Every route is guarded by an internal API key supplied in the `X-Internal-Api-Key` header. This is
not optional: when embedded, the router serves a single `HttpServer` on its public port, so
anything mounted there is publicly reachable, and an unguarded route would let anyone who can
reach the router send alerts through this service.

Auth is chosen **per route**, as a required argument to `services::server_wrap`, mirroring the
router's own idiom. A new route cannot silently skip authentication — omitting it is a compile
error, and `auth::NoAuth` is the explicit, greppable opt-out.

Health check endpoints are served from a separate unguarded scope, since probes do not carry
credentials.


One known ordering: actix runs a body extractor before the handler, and the guard runs inside the
handler, so a request whose JSON does not parse is answered `400` without its key being checked.
Accepted rather than fixed — the alternative is extracting raw bytes and deserializing by hand,
trading typed extraction for the concealment of a documented schema. `tests/notify.rs` asserts it so
it stays a recorded property.

`text`, `subject` and `body` are `Secret<String>`, so redaction is the type's job rather than a
hand-written `Debug` that someone has to remember to update when a field is added. Sizes still
reach the logs from the client, which emits `chars` per request.

## The API

Four surfaces under one scope. **Delivery** sends a message; **configuration** reads and writes the
rows this plane owns — what an alert is, whether it runs, the mappers the dashboard reads, and how
far a user has read their notifications; **lifecycle** is the alert manager's own working state,
which nobody edits and which it rewrites every run; **instances** are the record of who each
announcement was about. Routes under `/alerts/config`, `/alerts/lifecycle`, `/alerts/instances` and
`/alerts/dimensions`, and `/health/ready`, touch the database; delivery routes do not.

The whole surface, guarded and not:

| Method | Path | Auth |
|---|---|---|
| `POST` | `/alerts/chat/notify/{destination}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/chat/upload/{destination}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/email/notify/{destination}` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/definitions` | `X-Internal-Api-Key` |
| `POST` | `/alerts/config/definitions` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/definitions/{id}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/config/definitions/{id}` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/mappers` | `X-Internal-Api-Key` |
| `POST` | `/alerts/config/mappers` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/mappers/{name}/{key}` | `X-Internal-Api-Key` |
| `DELETE` | `/alerts/config/mappers/{name}/{key}` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/enablement` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/enablement/{name}/{product}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/config/enablement/{name}/{product}` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/notifications/read` | `X-Internal-Api-Key` |
| `POST` | `/alerts/config/notifications/read` | `X-Internal-Api-Key` |
| `GET` | `/alerts/lifecycle/{channel}/state` | `X-Internal-Api-Key` |
| `POST` | `/alerts/lifecycle/{channel}/state` | `X-Internal-Api-Key` |
| `GET` | `/alerts/lifecycle/{channel}/announcements` | `X-Internal-Api-Key` |
| `POST` | `/alerts/lifecycle/{channel}/announcements` | `X-Internal-Api-Key` |
| `POST` | `/alerts/lifecycle/{channel}/announcements/{id}` | `X-Internal-Api-Key` |
| `GET` | `/alerts/instances/{channel}/{announcement_id}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/instances/{channel}/{announcement_id}` | `X-Internal-Api-Key` |
| `GET` | `/alerts/dimensions/{channel}/{announcement_id}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/dimensions/{channel}/{announcement_id}` | `X-Internal-Api-Key` |
| `GET` | `/health` | none — liveness |
| `GET` | `/health/ready` | none — readiness |

The scope is `/alerts` rather than `/observability`: it names the resource being posted, not the
service, so it stays correct as the crate widens past delivery.

### Delivery

Three delivery routes across two channels. **The path says where, the body says what** — the URL names the
channel and the destination, the body carries only content. Channel ids, recipient addresses and
credentials live in configuration, so a caller cannot address a channel that was not set up for it
and no credential travels on the wire.

```http
POST /alerts/chat/notify/{destination}
X-Internal-Api-Key: <key>

{ "text": "*3 merchants not converting*", "reply_to": "cmtk931s114h8c9mfodi4ou1s" }
→ 200 { "status": "delivered", "message_id": "cmtk931zk14lec9mf1svtd88t" }
```

```http
POST /alerts/chat/upload/{destination}
X-Internal-Api-Key: <key>
Content-Type: multipart/form-data

file=<required bytes>&filename=<optional override>&title=<optional>&comment=<optional>&reply_to=<optional message_id>
→ 200 { "status": "delivered", "file_id": "cmtmsn9c110b7s7g92e72zv1u" }
```

One file is accepted per call. Uploads use the provider's external three-call flow and can be shared
under the message named by `reply_to`; the upload itself returns a file id, not a message id. Bodies are capped by `chat.max_upload_bytes` (25 MiB by default). The multipart extractor buffers
an accepted file in memory, but accounts for the body as chunks arrive and rejects the chunk that
crosses the cap rather than first retaining an oversized body.

```http
POST /alerts/email/notify/{destination}
X-Internal-Api-Key: <key>

{ "subject": "3 merchants not converting", "body": "<pre>…</pre>" }
→ 200 { "status": "delivered" }
```

Keeping the destination in the path means it reaches access logs, metrics labels and tracing spans
without anyone parsing a body, so "which destination is failing" is answerable from the ops view.

`reply_to` takes a `message_id` from an earlier response and threads under it. It exists on chat
only; sending it to the email route is a `400`, not a silently dropped field, because a recovery
notice that quietly loses its link to the alert it clears is a bug nobody notices.

**Nothing here renders.** `text`, `subject` and `body` are delivered exactly as they arrive, so the
caller owns markup and escaping. That is deliberate: the reference alerting service already renders
a summary in Slack `mrkdwn` for chat and a full per-alert list for email, and those are not the same
message. `body` is **HTML**, because both email backends in `external_services` hardcode an HTML
body and there is no plain-text path to reach.

A single `POST /notify/{destination}` over a channel-tagged body was considered and rejected: the
destination already resolves the channel through configuration, so a tag in the body is a second
authority on the same fact and the two can disagree.

### The status code answers a different question from the body

**HTTP status says whether the notifier worked. `status` says whether the message arrived.**

A provider that refuses — for any reason, including `channel_not_found` or `token_revoked` — is a
`200` carrying `status: "refused"`. It was reached, it answered, and this service did its job. Only
a request we cannot act on, an unreachable provider, or our own fault is an error. So a `5xx` from
this service means it is genuinely broken, and an alert on `5xx` fires at no other time.

This is the line payments draws between a connector declining a transaction and a connector being
unreachable, and it is drawn by *what the provider said* rather than by whose fault it is. Whether a
bad channel id is our mistake or a merchant's depends on who owns the destination, and that moves
from a config file to a database row without a status code being able to move with it.

```http
→ 200 { "status": "refused", "error_code": "channel_not_found" }
→ 200 { "status": "refused", "error_code": "rate_limited", "retry_after_seconds": 30 }
```

`status` is **required** on every success response. A caller cannot deserialize one without
confronting whether the message arrived — the same trick `external_services` uses on the provider's
own `ok` field, and for the same reason: this shape's failure mode is a caller that reads `200` and
stops looking.

`error_code` is a stable snake_case code in the provider's vocabulary. It is never the `Display` of
an internal error, and it is not always the exact bytes the provider sent, since `external_services`
folds synonyms on the way in (`is_archived` arrives as `not_in_channel`). One condition always
yields one code.

The errors that remain are short:

| | Status | Code |
|---|---|---|
| Body did not parse | 400 | `IR_04` |
| Missing or wrong API key | 401 | `IR_01` |
| Unknown destination | 404 | `IR_02` |
| Provider unreachable, or its answer was outside its documented envelope | 502 | `HE_03` |
| We failed | 500 | `HE_00` |

One case is deliberately *not* an error: a provider that accepts the message without naming an id
returns `{"status": "delivered", "message_id": null}`. The alert went out, and only the ability to
thread under it was lost — reporting a failure there would invite a retry that posts it twice.

### Configuration

Four resources, backed by the observability plane's own Postgres: the **definition** of an alert,
the **enablement** switch that says whether it runs, the **mappers** the portal reads,
and the **notification watermark** the bell reads. All four answer in the envelope the delivery
routes use, and `status` carries the same weight on a state read as it does on a notification: a
caller cannot read a `200` and assume there was something there.

#### Definitions

**A definition is one row, not four resources.** Suppression, snooze and thresholds are `json`
columns of `alerts_info` rather than side tables, so they are fields of this resource. All three are
typed, so every value written through this API has a shape the alert manager reads back. Blacklist
and thresholds are lists of entries with named fields. Snooze is r-apps' own document, because that
is what the alert manager evaluates: an object keyed `snooze_entry_<time>` or
`custom_snooze_entry_<time>`, each entry carrying the dimension values it covers and
`snooze_end_time` (optionally `snooze_start_time`) as `YYYY-MM-DD HH:MM:SS` in IST. An entry that is
keyed otherwise, has no end time, or has a time in any other format is `400` `HE_03`: r-apps' reader
fails on an entry without an end time, so the plane refuses to store one. The cost is that the bytes are not preserved: a value read back has been through
`serde_json` twice, so an entry written without its optional fields comes back with their defaults
filled in. Storing the columns as opaque strings would preserve them exactly, and was rejected: the
failure it avoids is cosmetic, and the one it introduces — a dashboard writing a key nothing reads,
discovered when an alert silently stops being suppressed — is not. `metadata` and `comments` stay
free-form, because nothing in this plane interprets them.

**An update mentions only what it changes.** Three portal screens edit different parts of one row,
so a whole-row `PUT` from any of them would discard what the other two just saved. An absent field
is left alone, an explicit `null` clears it, and a value sets it; `is_enabled` cannot be cleared, so a
`null` for it leaves it unchanged. Optimistic concurrency was the
alternative and was rejected: two screens editing *different* columns are not in conflict, and
making them retry against each other is worse than the lost update it prevents.

```http
POST /alerts/config/definitions
{ "name": "sr_drop", "product": "payments", "is_enabled": true, "author": "reliability_team",
  "blacklist": [{ "merchant_id": "merchant_1234", "reason": "dead test merchant" }] }
→ 200 { "id": "0189…", "name": "sr_drop", "is_enabled": true, "blacklist": [ … ], … }

POST /alerts/config/definitions/0189…
{ "thresholds": [{ "merchant_id": "merchant_1234", "tolerance": 2.5 }] }
→ 200 the whole definition, with blacklist and snooze untouched
```

`is_enabled` and `author` are **required** on create. The column has no default, so a definition
created without saying would be off — which reads as "the alert is broken" rather than "nobody enabled
it"; and the internal API key names the calling service, not a person, so if the body does not say
who is asking then nothing does. `name` and `product` cannot be changed afterwards: they are the
alert's identity, referenced by the enablement table and matched by name in the alert manager, so a
rename through an update would orphan those references rather than failing.

`name` reserves one value. **`all` is the definition carrying suppression that applies to every
detector**, which is the only way to express "mute this merchant everywhere" now that suppression is
a column rather than a table. It is read, listed and edited like any other definition, and it is the
one thing that cannot have an enablement row — it is not a detector, so there is nothing for a
switch on it to turn on or off.

**A definition has no delete route.** `is_enabled` is how an alert is turned off; unlike a delete it
is reversible.

#### Two switches, and what each one gates

`alerts_info.is_enabled` and `merchants_alert_external_config.is_enabled` switch different things,
as they do in r-apps. **The definition decides whether a detector runs; the enablement row decides
whether its alerts are delivered to merchants.** A missing enablement row delivers nothing, matching
r-apps' inner join on the enabled rows.

```
effective = definition.is_enabled AND coalesce(enablement.is_enabled, false)
```

The definition decides whether a detector runs at all, so with it off there is no result to deliver
anywhere. The enablement row is the second gate, on merchant-facing delivery only; internal channels
ignore it. `effective_is_enabled` is therefore "this alert reaches merchants".

Both values are reported, because a caller that saw only the stored one could not tell "delivered to
merchants" from "switched on, but the definition is off":

```http
POST /alerts/config/enablement/sr_drop/payments
{ "is_enabled": true }
→ 200 { "name": "sr_drop", "is_enabled": true, "effective_is_enabled": false, … }
```

The write is a real upsert — one statement with `(name, product)`, the table's primary key, as its
conflict target — so a repeated call updates rather than adding a second row disagreeing with the
first. r-apps leaves this table keyless and permits exactly that. It also validates the pair against
`alerts_info` with a database trigger this schema does not have, so **the API checks the definition
exists**; without the check a switch can be wired to an alert nobody defined and looks on the screen
exactly like one that works.

#### The mappers

The option lists and labels behind the portal's mappers screen, keyed on `(name, key_)`.

```http
POST /alerts/config/mappers
X-Internal-Api-Key: <key>
X-User-Name: ops@example.com

{ "name": "dashboard", "key": "slack_users", "values": "[]", "metadata": {"category": "dashboard"} }
→ 200 { "status": "saved", "entry": { "name": "dashboard", "key": "slack_users", … } }

GET    /alerts/config/mappers                        → 200 { "status": "found", "entries": [ … ] }
GET    /alerts/config/mappers/dashboard/slack_users  → 200 { "name": "dashboard", "key": "slack_users", … }
GET    /alerts/config/mappers/dashboard/unknown      → 404 HE_02
DELETE /alerts/config/mappers/dashboard/slack_users  → 200 { "status": "retired" }
DELETE /alerts/config/mappers/dashboard/unknown      → 404 HE_02
```

**A delete retires the live row rather than removing it.** The table's unique index is *partial* —
one enabled row per `(name, key_)` — so a save updates the live row in place, and a retired row stays
behind while a later save of the same key writes a new one. A save is therefore a single `INSERT … ON CONFLICT (name, key_) WHERE is_enabled IS TRUE DO UPDATE`, which
names the index's own predicate so Postgres can infer it. Both halves of that matter: an upsert
assuming a plain unique constraint fails outright, and one matching on `(name, key_)` without the
predicate finds a retired row and brings it back with its old value.

**`product`, `values_` and `metadata` are `json`, not `jsonb`, and are never re-encoded.** The
dashboard serializes them itself and the mappers screen parses some of them twice, so they cross
this service as raw bytes in both directions — see `diesel_models::observability::raw_json`.
Parsing into a `serde_json::Value` and serializing it again would hand the screen back a document it
did not save. A definition takes the opposite trade for the opposite reason: its `json` columns are
typed because the alert manager reads them, and nothing reads a mapper entry but the screen that
wrote it.

An entry's JSON is capped at 1 MiB. The dashboard decides
how large an entry is, and one oversized save becomes a row nothing can read back — a broken page
long after the save that caused it, rather than a rejected request naming the entry.

#### The notification watermark

The bell shows what happened after the watermark and hides what happened before it.

```http
POST /alerts/config/notifications/read
X-Internal-Api-Key: <key>
X-User-Name: ops@example.com

→ 200 { "status": "found", "last_read_at": "2026-09-09T12:34:56.789Z" }

GET /alerts/config/notifications/read  → 200 { "status": "absent", "last_read_at": null }
```

The write takes no body: the instant is this service's clock, not the caller's, so a skewed
dashboard cannot hide alerts nobody was shown — a watermark never moves backwards.

#### Who a request is for

The internal API key authenticates the **service**, not a person, so the two routes that need a user
— a mapper save and the watermark — read `X-User-Name`. **Nothing authenticates it**; it is an
assertion by a caller that has already decided who it is acting for. The definition resource asks
instead for `author` in the body, because a definition records who wrote it rather than who is
looking at it.

That is the honest shape of the deployment. Local accounts are disabled in sandbox and production
alike (`localUsers: false`), so every request arrives with no name, the watermark table holds one
shared row, and mapper saves are attributed to `reliability_team`. Keeping the name
on the request anyway is what makes that a data fact rather than a schema one: the day the portal
authenticates, the alert manager forwards the name and rows appear per person with no route and no
migration to change.

A header rather than a path segment, which is where this crate otherwise puts what a request is
about. The empty name every caller sends today has no spelling as a path segment, so the path form
could not express the state that actually exists, and a user name is an email address wherever there
is one, which a path would write into every access log. An absent header is the empty name; a header
that is not UTF-8 is a `400`, because falling back would file one person's watermark under the
shared row.

#### An empty list and an unset watermark are answers, not a `404`

A mapper list with no entries is `200` with `status: "absent"`, and a watermark that was never set is
`200` with `status: "absent"`. Both screens have a defined behaviour for "nothing saved yet" — offer
the built-in options, treat everything as unread — and making that an HTTP error would mean the
caller has to treat an error response as normal, which is the habit that hides a real one.

`404` keeps its meaning: a path naming something this service does not have. An unconfigured
destination, an unknown definition id, an unknown enablement key and a mapper entry that does not
exist are all `404`.

#### An empty answer is never an outage

A store that answered nothing and a store that could not be asked must not look the same: the alert
manager's own outage rule reads "no alerts" as "nothing is wrong", so collapsing the two would
report all-clear during exactly the incident this plane exists to notice. A list with no rows is a
`200` with a count of zero; a list that could not be read is a `500`, or a `503` when no database
connection could be taken at all.

The configuration errors, added to the table above:

| | Status | Code |
|---|---|---|
| Definition already exists for this name and product | 400 | `HE_01` |
| Name and product do not identify an alert (or name the reserved `all` row) | 400 | `HE_03` |
| A snooze entry is not keyed `snooze_entry_`/`custom_snooze_entry_`, or has no readable end time | 400 | `HE_03` |
| Mapper entry larger than this service stores | 400 | `HE_03` |
| Unknown definition id | 404 | `HE_02` |
| Unknown enablement key | 404 | `HE_02` |
| A query against the observability database failed | 500 | `HE_00` |
| Observability database unreachable | 503 | `HE_00` |

`503` only when no connection can be taken, for the reason `/health/ready` uses it: the service is
fine, and the condition is expected to clear without anyone touching it. A query that fails once
connected is a `500`. The failing host, database and role
reach the log and never the response.

An empty `name` or `key`, one wider than its column, and an unreadable `X-User-Name` are `IR_04`
alongside a body that did not parse. The column widths are checked here rather than left to
Postgres, which rejects the same values as an opaque failure with a `500` attached.

### Lifecycle

Two resources, and the split between them is the point. `alerts_intermediate` holds **what is
firing now**; `alerts_main` records **what was actually said**, one row per announcement, with its
`sent` flag and the thread it opened. Collapsing them is why the service this replaces cannot say
whether an alert was delivered.

`{channel}` is `slack` or `xyne`. Both channels share `alerts_main` and `alerts_intermediate`, each
row carrying its `channel`, and every read, replace and lock is scoped to the channel in the path.
A segment that is neither is a `404` with no error body, the same answer as a path this service
does not serve, rather than a fallback to one of them or an empty store. A state write whose
`id_intermediate` belongs to the other channel is a `400` and changes nothing.

**Two write shapes, deliberately not alike.** A state write is a replacement: what it does not
carry is removed. An announcement write is an append: it adds a row and takes nothing away. There
is no `DELETE` on either, and a state write never touches `alerts_main` — a state row references an
announcement `ON DELETE CASCADE`, so a replace that reached the announcement table would delete
state rows pointing at it, including ones the same request is writing.

**The dashboard's alert list is the announcement log.** `GET .../announcements?start=&end=` lists the
announcements whose `ts_alert` falls in the window, newest first, as r-apps' `getAlerts` reads
`alerts_main`. With no bounds the window is the last 7 days; a window that ends before it starts or
spans more than 30 days is refused, matching r-apps' `DEFAULT_DAYS` and `DAYS_LIMIT`.
`POST .../announcements/{id}` with `{"metadata": {...}}` is r-apps' `updateAlert`: it replaces the
announcement's `metadata` (resolution, comments and the rest) and merges the same keys, except
`is_visible_to_merchant`, into the state rows that reference it, in one transaction. It does not
touch `last_updated_at` on those state rows, so an edit from the dashboard never turns the alert
manager's next state write into a `409`.

**Rows are addressed by `id_intermediate`, and a caller echoes back the ids it read.** A row whose
id is not echoed back is removed and, if it is still firing, written again as a new row — which
loses the episode's start and its thread. An alert the caller has just detected has no id to send,
and the handler mints one. The response's `id_intermediates` lists the id of every row written, in
request order, so a second write in the same run can echo them back.

**Every write stamp is this service's clock.** `last_updated_at`, and an announcement's `ts_alert`,
are set by the handler: the columns carry no `DEFAULT`, so somebody has to choose, and a caller
minutes out of step would make an episode look older than it is. The episode times a caller sends —
`ts_alert`, `latest_ts_alert`, `recovered_ts` — are stored as sent, and `expected_last_updated_at`
is compared, never stored.

A field the caller leaves out is stored as r-apps stores it rather than as `NULL`: `dimensions` as
`[]`, `rca_metadata` as `{}`, `max_duration` and `duration` as `0`, `group_id` and `priority` as
`''`, `sent` and `critical` as false, and a state row's `ts_alert` and `latest_ts_alert` as the time
of the write.

#### Two overlapping runs

The cron fires every fifteen minutes, so a slow run means two whole-state writes in flight. A write
carries `expected_last_updated_at` — the value the read handed out — and is applied only if the
stored state still matches it; a mismatch is `409` and nothing is written. Absent or `null` asserts
that the state was empty at read time, so a forgotten precondition fails closed rather than
overwriting whatever is there. A write that stores no alerts answers `last_updated_at: null`, and
that is what the next write sends.

Timestamps cross the wire in milliseconds, so the stored value is compared in milliseconds too. A
row stamped with finer precision by anything else would otherwise never match what a read handed
out, and every write would be refused.

The precondition alone is not enough when the two writes genuinely overlap: both would read the
same value before either wrote. Each write takes a Postgres advisory lock on its channel first, so
the check happens against state nothing else is changing. The loser waits, then is refused.

A `409` is not a `400`: the body was fine and would have been accepted a moment earlier. The
caller's move is to read the state again — never to retry the write it just sent, which is the
stale one.

#### The size of a write

One whole-state write carries at most 5,000 alerts. Because a write replaces everything, the same
number bounds the stored state and the read that returns all of it. The lifecycle routes accept a
body of up to 16 MiB, rather than the 2 MiB every other route keeps, so a write at the cap fits.

**Over the cap the whole write is refused and nothing is applied.** Truncating it would drop alerts
the caller believes are recorded and re-announce them on the next run, which is the failure the cap
exists to prevent rather than one it should cause.

The lifecycle errors, added to the tables above:

| | Status | Code |
|---|---|---|
| Whole-state write over 5,000 alerts | 400 | `HE_03` |
| A state row's `id_intermediate` belongs to the other channel | 400 | `HE_03` |
| A state row references an announcement this channel does not have | 404 | `HE_02` |
| The state changed after it was read | 409 | `IR_16` |
| Announcement window ends before it starts or spans more than 30 days | 400 | `HE_03` |
| Unknown announcement id on this channel | 404 | `HE_02` |

A value wider than its column — `name`, `product`, `group_id` and `priority` are `VARCHAR(64)`,
`ts_slack` is `VARCHAR(255)` — and the same `id_intermediate` sent twice in one write are `IR_04`,
alongside a body that did not parse. Both are checked before the query runs: Postgres rejects the
first as an opaque `22001` and refuses the second with a message about the statement rather than
about the request, and either would fail the whole batch.

### Instances

`merchants_alert_external` holds one row per merchant an announcement was about, and
`merchants_alert_external_dimension` one row per dimension value (connector, payment method, ...)
behind it. Both reference `alerts_main` `ON DELETE CASCADE`, so removing an announcement removes its
rows; no route here removes one.

The announcement is the `{announcement_id}` path segment and no body field names it. `{channel}` is
`slack` or `xyne`; both channels share both tables, each row carrying its `channel`, and every read,
replace and announcement lookup is scoped to the channel in the path. An id with no announcement on
that channel is `404` `HE_02` on a read and on a write, and a write refused this way changes
nothing. A segment that is not a known channel, or an id that is not a UUID, is an empty `404`, as
under `/lifecycle`.

A read answers `{count, merchants}` or `{count, dimensions}`: instances ordered by `merchant_id`,
dimensions by `dimension_key` then `dimension_value`, both then by `id_merchant_table`.

A write replaces every row its announcement carries on that channel in that table, and answers
`{stored, removed, ts_alert}`: the rows inserted, the rows it deleted, and the `ts_alert` the new
rows carry, or `null` when it stored none. An empty write clears the announcement's rows. The
delete and the insert run in one transaction that first takes a Postgres advisory lock on the
announcement, one lock per table, so two writes for one announcement run one after the other and
the second replaces what the first stored.

`ts_alert` and `last_updated_at` are the time of the write, truncated to milliseconds, and
`id_merchant_table` is minted with `common_utils::generate_uuid_v7()`. A field the caller leaves out
is stored as r-apps stores it: `name`, `product`, `merchant_id`, `dimension_key`,
`dimension_value`, `attribution`, `priority` and `tenant_id` as `''`; `slack_info`,
`communication_info` and `metadata_alert_details` as `{}`; `dimensions`, `auxiliary_dimensions` and
`metadata` as the JSON string `"{}"`; `is_visible` as `true`; and `ts_slack` as the announcement's
`ts_slack`, or `''` when it has none. `current_metric`, `expected_metric`, `max_duration`,
`start_time`, `latest_ts_alert`, `recovered_ts` and `id_intermediate` are stored as sent, `NULL` when
absent.

#### The size of a write

One instance write carries at most 500 merchants and one dimension write at most 500 dimensions, so
the same numbers bound what one announcement holds. A write over its cap is refused whole. At the
cap a write is a single insert of 500 rows of 27 columns, inside Postgres' 65,535 bind parameters.

Both write routes take the `/alerts` scope's 2 MiB body limit, so 500 rows fit while they average
under about 4 KiB of JSON each. A larger body is refused as `IR_06`, the same answer as a body that
does not parse, before the caps or widths are checked.

Column widths are counted in characters and checked on every row before a database connection is
taken: `name`, `product`, `merchant_id`, `dimension_key`, `priority` and `tenant_id` are
`VARCHAR(64)`; `attribution`, `dimension_value` and `ts_slack` are `VARCHAR(255)`.

The instance errors, added to the tables above:

| | Status | Code |
|---|---|---|
| Body over 2 MiB, or did not parse | 400 | `IR_06` |
| A value wider than its column | 400 | `IR_07` |
| Instance write over 500 merchants, or dimension write over 500 dimensions | 400 | `HE_03` |
| Unknown announcement id on this channel | 404 | `HE_02` |

## Destinations

Configured under `chat.destinations.<id>` and `email.destinations.<id>`, resolved once at boot.

**Ids set from the environment arrive lowercased and cannot contain `__`.** The `config` crate
lowercases every environment key before splitting it, and `__` is the level separator, so
`OBSERVABILITY__CHAT__DESTINATIONS__SR_ALERTS__CHANNEL` sets
`chat.destinations.sr_alerts.channel` and there is no spelling that yields `SR_ALERTS`. The
service refuses to start on an id that would not survive the round trip, rather than failing to
match it at lookup time.

A chat destination is tagged `xyne`, `slack` or `log`. Xyne and Slack are one client differing in
base URL and credential, not two integrations. `log` accepts messages and delivers nothing, so the
whole path can be exercised before real credentials exist. It logs sizes and never the message,
since it writes to the same stream as everything else and an alert body carries merchant ids and
payment volumes.

Having **no** destinations is a warning, not a boot failure: a first deployment has none until
credentials exist, and refusing to start would make the service undeployable before them. A
destination that is configured but cannot be built *is* a boot failure, because dropping it would
leave the service answering "unknown destination" to something that is very much configured.

Email destinations share **one transport**, configured under `[email]` using
`external_services`' own settings, so the SES / SMTP / no-email selection and its validation are the
router's rather than a second copy. Unlike chat, where a destination *is* an endpoint with its own
credential, email is one transport and many addresses.

`NO_EMAIL_CLIENT` is the default and the off switch — a backend that sends nothing already is one,
so there is no separate "email enabled" flag. The transport is validated at boot only when
destinations exist, so a service with none does not need a verified sender to start, and a
destination with no address fails the boot rather than accepting alerts and sending them nowhere.

Two limits are inherited from `external_services::email`, both tracked separately: an email
destination holds one address, so reaching three people is three destinations, and the body must be
HTML. Related: `EmailError` has no refusal vocabulary — a rejected recipient, a throttle and an
unverified sender all arrive as one variant — so email only ever reports `delivered` or fails.
`status: "refused"` is reachable for chat and not yet for email.

## Layout

```
routes/          the route tree, and one module of handlers per area: notify and config
core/            what one request does, per area: deliver a message, or read and write configuration
domain/          what delivering an alert is: the notifier traits and the types they exchange
types/           the wire contract, per area
```

The two concerns are separated by module rather than by directory. `notify` delivers a message and
keeps nothing; `config` reads and writes a configuration row and sends nothing. They share the HTTP
server, the database pool, authentication, `server_wrap` and the error types. The one deliberate exception is `routes/app.rs`, which holds *every* route this service
serves — both concerns' — so the tree and its guards are one file rather than a search.

Rows and their queries are not here at all: `alerts_info`, `merchants_alert_external_config`,
`alerts_dicts`, `notification_reads`, `alerts_main`, `alerts_intermediate`,
`merchants_alert_external` and `merchants_alert_external_dimension` are modelled in
`diesel_models::observability`, alongside every other table this database owns, so the alert
manager and this service read one definition of them rather than two.

The configuration areas have no `domain` types: a row is a row, and its types are
`diesel_models::observability` on one side and `types/` on the other. A trait between them would
abstract over one implementation.

`domain` holds no HTTP. `core` holds no traits. A handler that grows logic belongs in `core`; a
concept that a background job would also need belongs in `domain`.
