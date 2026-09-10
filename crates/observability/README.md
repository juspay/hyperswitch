# observability

The observability plane for Hyperswitch.

`observability` is the home for alert *delivery* and for the alert manager's own *state*. Deciding
what is alert-worthy — thresholds, detectors, suppression — is **not** done here; alerts arrive
already decided, and storing a threshold is not applying it.

Two concerns live here today. The [`notifier`](src/domain/notifier.rs) receives alert data over a
webhook and delivers it to a channel. The configuration routes own the rows the alert manager reads:
what an alert is and whether it runs, together with what it used to write into the application's
ClickHouse — the mappers dictionary and the notification bell's read watermark.

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

Four surfaces under one scope. **Delivery** sends a message; **configuration** reads and writes
the rows somebody edits — what an alert is, whether it runs, the mappers the dashboard reads, and
how far a user has read their notifications; **lifecycle** is the alert manager's own working
state, which nobody edits and which it rewrites every run; **instances** are the record of who
each announcement was about. A route under `/alerts/config`, `/alerts/lifecycle`,
`/alerts/instances` or `/alerts/dimensions` touches the database and nothing else does.

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
| `GET` | `/alerts/config/dictionary` | `X-Internal-Api-Key` |
| `POST` | `/alerts/config/dictionary` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/dictionary/{name}/{key}` | `X-Internal-Api-Key` |
| `DELETE` | `/alerts/config/dictionary/{name}/{key}` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/enablement` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/enablement/{name}/{product}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/config/enablement/{name}/{product}` | `X-Internal-Api-Key` |
| `GET` | `/alerts/config/notifications/read` | `X-Internal-Api-Key` |
| `POST` | `/alerts/config/notifications/read` | `X-Internal-Api-Key` |
| `GET` | `/alerts/lifecycle/{channel}/state` | `X-Internal-Api-Key` |
| `POST` | `/alerts/lifecycle/{channel}/state` | `X-Internal-Api-Key` |
| `POST` | `/alerts/lifecycle/{channel}/announcements` | `X-Internal-Api-Key` |
| `GET` | `/alerts/instances/{channel}/{announcement_id}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/instances/{channel}/{announcement_id}` | `X-Internal-Api-Key` |
| `GET` | `/alerts/dimensions/{announcement_id}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/dimensions/{announcement_id}` | `X-Internal-Api-Key` |
| `GET` | `/health` | none — liveness |

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
the **enablement** switch that says whether it runs, the **mappers dictionary** the portal reads,
and the **notification watermark** the bell reads. All four answer in the envelope the delivery
routes use, and `status` carries the same weight on a state read as it does on a notification: a
caller cannot read a `200` and assume there was something there.

#### Definitions

**A definition is one row, not four resources.** Suppression, snooze and thresholds are `json`
columns of `alerts_info` rather than side tables, so they are fields of this resource. All three are
typed — a list of entries with named fields — so a caller cannot store a shape the alert manager
will fail to read. The cost is that the bytes are not preserved: a value read back has been through
`serde_json` twice, so an entry written without its optional fields comes back with their defaults
filled in. Storing the columns as opaque strings would preserve them exactly, and was rejected: the
failure it avoids is cosmetic, and the one it introduces — a dashboard writing a key nothing reads,
discovered when an alert silently stops being suppressed — is not. `metadata` and `comments` stay
free-form, because nothing in this plane interprets them.

**An update mentions only what it changes.** Three portal screens edit different parts of one row,
so a whole-row `PUT` from any of them would discard what the other two just saved. An absent field
is left alone, an explicit `null` clears it, and a value sets it. Optimistic concurrency was the
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

`is_enabled` and `author` are **required** on create. The column defaults to false, so a definition
created without saying is off — which reads as "the alert is broken" rather than "nobody enabled
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
is reversible, and deleting an `alerts_info` row cascades to every `alerts_main` row referencing it,
destroying the record of what was announced in order to stop announcing it.

#### Two switches, and which wins

`alerts_info.is_enabled` and `merchants_alert_external_config.is_enabled` are both switches on the
same alert. **The definition is the master switch; the enablement row can only narrow it.**

```
effective = definition.is_enabled AND coalesce(enablement.is_enabled, true)
```

The definition decides whether a detector runs at all, so with it off there is no result for an
enablement row to publish. Letting the narrower table win would mean an operator disabling a
definition could be silently overridden from a screen they were not looking at, which is what a
master switch exists to prevent. A missing enablement row narrows nothing, matching the column's
`DEFAULT TRUE`, so adding a definition is enough to make it run. Most-recently-updated-wins was the
alternative and was rejected: it makes the answer depend on clock skew between two writers and
offers no way to say "off, and stay off".

Both values are reported, because a caller that saw only the stored one could not tell "on" from
"on, but the definition is off":

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

#### The mappers dictionary

The option lists and labels behind the portal's mappers screen, keyed on `(name, key_)`.

```http
POST /alerts/config/dictionary
X-Internal-Api-Key: <key>
X-User-Name: ops@example.com

{ "name": "dashboard", "key_": "slack_users", "values_": "[]", "metadata": {"category": "dashboard"} }
→ 200 { "status": "saved", "entry": { "name": "dashboard", "key_": "slack_users", … } }

GET    /alerts/config/dictionary                    → 200 { "status": "found",  "entries": [ … ] }
GET    /alerts/config/dictionary/dashboard/unknown  → 200 { "status": "absent", "entry": null }
DELETE /alerts/config/dictionary/dashboard/unknown  → 200 { "status": "absent" }
```

**`alerts_dicts` keeps history.** A delete retires the live row rather than removing it, and the
table's unique index is *partial* — one enabled row per `(name, key_)`, superseded rows kept. A save
is therefore a single `INSERT … ON CONFLICT (name, key_) WHERE is_enabled IS TRUE DO UPDATE`, which
names the index's own predicate so Postgres can infer it. Both halves of that matter: an upsert
assuming a plain unique constraint fails outright, and one matching on `(name, key_)` without the
predicate finds a retired row and brings it back with its old value.

**`product`, `values_` and `metadata` are `json`, not `jsonb`, and are never re-encoded.** The
dashboard serializes them itself and the mappers screen parses some of them twice, so they cross
this service as raw bytes in both directions — see `diesel_models::observability::raw_json`.
Parsing into a `serde_json::Value` and serializing it again would hand the screen back a document it
did not save. A definition takes the opposite trade for the opposite reason: its `json` columns are
typed because the alert manager reads them, and nothing reads a dictionary entry but the screen that
wrote it.

An entry's JSON is capped by `dictionary.max_entry_bytes` (1 MiB by default). The dashboard decides
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
— a dictionary save and the watermark — read `X-User-Name`. **Nothing authenticates it**; it is an
assertion by a caller that has already decided who it is acting for. The definition resource asks
instead for `author` in the body, because a definition records who wrote it rather than who is
looking at it.

That is the honest shape of the deployment. Local accounts are disabled in sandbox and production
alike (`localUsers: false`), so every request arrives with no name, the watermark table holds one
shared row, and dictionary saves are attributed to the `username` column's default. Keeping the name
on the request anyway is what makes that a data fact rather than a schema one: the day the portal
authenticates, the alert manager forwards the name and rows appear per person with no route and no
migration to change.

A header rather than a path segment, which is where this crate otherwise puts what a request is
about. The empty name every caller sends today has no spelling as a path segment, so the path form
could not express the state that actually exists, and a user name is an email address wherever there
is one, which a path would write into every access log. An absent header is the empty name; a header
that is not UTF-8 is a `400`, because falling back would file one person's watermark under the
shared row.

#### Nothing stored is an answer, not a `404`

A dictionary read or a watermark read that finds nothing is `200` with `status: "absent"`. Both
screens have a defined behaviour for "nothing saved yet" — offer the built-in options, treat
everything as unread — and making that an HTTP error would mean the caller has to treat an error
response as normal, which is the habit that hides a real one.

`404` keeps its meaning: a path naming something this service does not have. An unconfigured
destination, an unknown definition id and an unknown enablement key are all `404`, and none of them
is a state a screen expects — a caller that sent an id got it from a list this service returned.

#### An empty answer is never an outage

A store that answered nothing and a store that could not be asked must not look the same: the alert
manager's own outage rule reads "no alerts" as "nothing is wrong", so collapsing the two would
report all-clear during exactly the incident this plane exists to notice. A list with no rows is a
`200` with a count of zero; a list that could not be read is a `503`.

The configuration errors, added to the table above:

| | Status | Code |
|---|---|---|
| Definition already exists for this name and product | 400 | `IR_05` |
| Name and product do not identify an alert (or name the reserved `all` row) | 400 | `IR_07` |
| Dictionary entry over `dictionary.max_entry_bytes` | 400 | `IR_08` |
| Unknown definition id | 404 | `IR_03` |
| Unknown enablement key | 404 | `IR_06` |
| Observability database unreachable | 503 | `HE_01` |

`503` rather than `500`, for the reason `/health/ready` uses it: the service is fine, and the
condition is expected to clear without anyone touching it. The failing host, database and role
reach the log and never the response.

An empty `name` or `key_`, one wider than its column, and an unreadable `X-User-Name` are `IR_04`
alongside a body that did not parse. The column widths are checked here rather than left to
Postgres, which rejects the same values as an opaque failure with a `500` attached.

### Lifecycle

Two resources, and the split between them is the point. `alerts_intermediate` holds **what is
firing now**; `alerts_main` records **what was actually said**, one row per announcement, with its
`sent` flag and the thread it opened. Collapsing them is why the service this replaces cannot say
whether an alert was delivered.

`{channel}` is `slack` or `xyne`. The tables come once per delivery channel — `alerts_main` and
`alerts_main_xyne`, and their `alerts_intermediate` twins — so the channel is a path segment. A
segment that is neither is a `404` rather than a fallback to one of them.

**Two write shapes, deliberately not alike.** A state write is a replacement: what it does not
carry is removed. An announcement write is an append: it adds a row and takes nothing away. There
is no `DELETE` on either, and a state write never touches `alerts_main` — a state row references an
announcement `ON DELETE CASCADE`, so a replace that reached the announcement table would delete
state rows pointing at it, including ones the same request is writing.

**Rows are addressed by `id_intermediate`, and a caller echoes back the ids it read.** A row whose
id is not echoed back is removed and, if it is still firing, written again as a new row — which
loses the episode's start and its thread. An alert the caller has just detected has no id to send,
and the handler mints one; the response and the next read carry it.

**Every stored timestamp is this service's clock.** The columns carry no `DEFAULT`, so somebody has
to choose, and durations here are computed by subtracting these timestamps from each other — a
caller minutes out of step would report an episode as older than it is and then write that back.
The one timestamp a caller sends is `expected_last_updated_at`, which is not stored.

#### Two overlapping runs

The cron fires every fifteen minutes, so a slow run means two whole-state writes in flight. A write
carries `expected_last_updated_at` — the value the read handed out — and is applied only if the
stored state still matches it; a mismatch is `409` and nothing is written. Absent or `null` asserts
that the state was empty at read time, so a forgotten precondition fails closed rather than
overwriting whatever is there.

The precondition alone is not enough when the two writes genuinely overlap: both would read the
same value before either wrote. Each write takes a Postgres advisory lock on its channel first, so
the check happens against state nothing else is changing. The loser waits, then is refused.

A `409` is not a `400`: the body was fine and would have been accepted a moment earlier. The
caller's move is to read the state again — never to retry the write it just sent, which is the
stale one.

#### The size of a write

`lifecycle.max_alerts` (5000 by default) caps the alerts one whole-state write may carry. Because a
write replaces everything, the same number bounds the stored state and the read that returns all of
it.

**Over the cap the whole write is refused and nothing is applied.** Truncating it would drop alerts
the caller believes are recorded and re-announce them on the next run, which is the failure the cap
exists to prevent rather than one it should cause.

The lifecycle errors, added to the tables above:

| | Status | Code |
|---|---|---|
| Whole-state write over `lifecycle.max_alerts` | 400 | `IR_10` |
| A state row references an announcement that does not exist | 400 | `IR_12` |
| Unknown channel | 404 | `IR_09` |
| The state changed after it was read | 409 | `IR_11` |
| Lifecycle state unreadable | 503 | `HE_01` |

A value wider than its column — `name`, `product`, `group_id` and `priority` are `VARCHAR(64)`,
`ts_slack` is `VARCHAR(255)` — and the same `id_intermediate` sent twice in one write are `IR_04`,
alongside a body that did not parse. Both are checked before the query runs: Postgres rejects the
first as an opaque `22001` and refuses the second with a message about the statement rather than
about the request, and either would fail the whole batch.

### Instances

Who an announcement was about. `merchants_alert_external` holds one row per affected merchant and
`merchants_alert_external_dimension` holds the breakdown behind it — one row per connector, payment
method, or whatever the detector split on.

**The announcement is a path segment and is the only way to address these rows.** Both tables
reference `alerts_main` `ON DELETE CASCADE`, so a caller records an announcement, gets an id back,
and posts to that id. There is no route that takes the reference in a body, so a row pointing at an
announcement that does not exist — or at none at all — is not expressible. An id that names no
announcement is `IR_12`, the same code the lifecycle write answers for the same condition, checked
before anything is written rather than left to arrive as an opaque constraint failure.

Removing an announcement takes its instances and its breakdown with it. Nothing this service
exposes removes one.

`{channel}` is `slack` or `xyne`, exactly as it is under `/lifecycle`. **The breakdown has no
channel**: `merchants_alert_external_dimension` exists once and references `alerts_main`, unlike
the instance table which comes once per delivery channel. Putting a channel in its path would
promise a `_xyne` breakdown table that does not exist, and answering "none" for it would read as an
alert that had no breakdown.

**A write replaces what its announcement carries.** A rerun of the same alert manager pass records
the same merchants once rather than twice, and an empty write clears them. The response reports
`removed` for the same reason the lifecycle write does: a replacement that removed far more than
expected is the shape of a caller that lost its own copy.

#### What is wrong, generically

`current_metric` against `expected_metric` is the generic form of "what is wrong", which holds for
success rate, volume, refunds and anything later where `sr`/`failed`/`total` did not.

**Absent is not zero.** The absolutes — zero volume, zero success — have no expected value and send
none. Storing `0` for them would read as "observed 0, expected 0", which is a healthy row. Both
columns are nullable and both are stored exactly as they arrive.

`ts_slack` is the other value that may honestly be absent. A caller that sends none takes the
announcement's thread, so nobody carries it around by hand; an instance recorded before its
announcement reached a channel has no thread anywhere and stores `null`. The column was `NOT NULL`
in the schema this model came from, which is what made that unrepresentable.

#### The size of a write

`instances.max_merchants` and `instances.max_dimensions` (500 each by default) cap the rows one
announcement may hold.

**Over the cap the write is cut down to it and stored, not refused** — the opposite of
`lifecycle.max_alerts`, and deliberately so. One alert across many connectors becomes many rows, so
the cap is reached during a *broad* outage, which is exactly when the record of who was affected
matters most; losing the whole write then is the wrong failure.

Two things make the cut safe to reason about:

* **What survives is the worst of it.** Rows are ordered by impact before the cut: an absolute
  first, since it expected nothing and everything is gone, then by distance from what was expected,
  and a row that reported no metrics at all last. Scoring the absolute as `expected - current` with
  both defaulted to `0` would rank a total outage as *no impact* and drop it first.
* **The cut is recorded on every row it kept**, under `truncated_by_impact` in
  `metadata_alert_details`, as well as in the response's `truncated`. A row found on its own says
  both that the breakdown is partial and by how much, so a shortened breakdown never reads as a
  narrower outage. The caller's own document is added to, never replaced.

Neither cap may exceed 2000: these tables are 25 and 26 columns wide and Postgres accepts 65535
bind parameters in one statement, so a batch insert stops working above roughly 2500 rows whatever
anyone configures. A cap outside the range fails the boot.

The instance errors, added to the tables above:

| | Status | Code |
|---|---|---|
| The announcement in the path does not exist | 400 | `IR_12` |
| Unknown channel | 404 | `IR_09` |
| Instances unreadable | 503 | `HE_01` |

A value wider than its column — `name`, `product`, `merchant_id`, `dimension_key`, `priority` and
`tenant_id` are `VARCHAR(64)`, `attribution`, `dimension_value` and `ts_slack` are `VARCHAR(255)` —
is `IR_04`, alongside a body that did not parse, and is checked over every row the request carried
including ones the cap will drop.

**The defaults these columns lost, supplied here instead:** `id_merchant_table` had
`gen_random_uuid()` and is minted with `uuid::Uuid::now_v7()`; `is_visible` had `DEFAULT TRUE` and
is set explicitly, so a caller that said nothing does not store a row nobody can see; `ts_alert`
had `CURRENT_TIMESTAMP` and is this service's clock, as `last_updated_at` is.

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
routes/          the route tree, and the notifier's handlers
core/            what one notify request does: resolve a destination and deliver
domain/          what delivering an alert is: the notifier traits and the types they exchange
alert_manager/   the alert manager's own state, with its own core/, routes/ and types/
```

The two concerns are separated by that last directory rather than by a filename. Everything outside
`alert_manager/` delivers a message and keeps nothing; everything inside it reads and writes a
configuration row and sends nothing. They share the HTTP server and the database pool, and nothing
else. The one deliberate exception is `routes/app.rs`, which holds *every* route this service
serves — both concerns' — so the tree and its guards are one file rather than a search.

Rows and their queries are not here at all: `alerts_info`, `merchants_alert_external_config`,
`alerts_dicts`, `notification_reads`, `alerts_main` and `alerts_intermediate` are modelled in
`diesel_models::observability`, alongside every other table this database owns, so the alert
manager and this service read one definition of them rather than two. The two lifecycle tables come
once per delivery channel, so their models are generated per table and hand back a
channel-agnostic row — the handlers take the channel as an argument and are written once.

`alert_manager` has no `domain/`: a row is a row, and its types are `diesel_models::observability`
on one side and `alert_manager/types/` on the other. A trait between them would abstract over one
implementation.

`domain` holds no HTTP. `core` holds no traits. A handler that grows logic belongs in `core`; a
concept that a background job would also need belongs in `domain`.
