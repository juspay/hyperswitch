# observability

The observability plane for Hyperswitch.

`observability` is the home for alert *delivery* and for the alert manager's own *state*. Deciding
what is alert-worthy — thresholds, detectors, suppression — is **not** done here; alerts arrive
already decided, and storing a threshold is not applying it.

Two concerns live here today. The [`notifier`](src/domain/notifier.rs) receives alert data over a
webhook and delivers it to a channel. The configuration routes own the rows the alert manager reads:
what an alert is, whether it runs and its per-merchant thresholds, together with what it used to
write into the application's ClickHouse — the mappers and the notification bell's read watermark.

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

`observability` uses no version-flavoured type. Its `v1` (default) and `v2` features only select
the flavour its shared dependencies compile with: `common_utils`, `diesel_models`,
`external_services` and `hyperswitch_interfaces`.

## Configuration

Reads `config/observability.toml` by default; override with `-f <path>`. Every value can be
overridden by an environment variable prefixed `OBSERVABILITY__`, with `__` separating levels — so
`OBSERVABILITY__AUTH__INTERNAL_API_KEY` sets `auth.internal_api_key`.

Configuration is validated at boot and startup fails loudly on a missing internal API key, rather
than on the first request.

`[database]` is the Postgres database holding the observability tables. As with `drainer`, the
service refuses to start without its `dbname`, `username` and `password`; `host` and `port` default
to `localhost:5432`, `pool_size` to 5 and `connection_timeout` to 10 seconds. The pool connects
lazily, so an unreachable database does not stop the service from starting; it shows as `503` on
`/health/ready`.

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

Two surfaces under one scope. **Delivery** sends a message; **configuration** reads and writes the
rows this plane owns — what an alert is, whether it runs, its per-merchant thresholds, the mappers the
dashboard reads, and how far a user has read their notifications. Routes under `/alerts/config`, and
`/health/ready`, touch the database; delivery routes do not.

### Delivery

Three delivery routes across two channels. **The path says where, the body says what** — the URL names the
channel and the destination, the body carries only content. Channel ids, recipient addresses and
credentials live in configuration, so a caller cannot address a channel that was not set up for it
and no credential travels on the wire.

The whole surface, guarded and not:

| Method | Path | Auth |
|---|---|---|
| `POST` | `/alerts/chat/notify/{destination}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/chat/upload/{destination}` | `X-Internal-Api-Key` |
| `POST` | `/alerts/email/notify/{destination}` | `X-Internal-Api-Key` |
| `GET` | `/health` | none — liveness |
| `GET` | `/health/ready` | none — readiness: `200` when a database connection is taken within 900 ms, otherwise `503` |

The scope is `/alerts` rather than `/observability`: it names the resource being posted, not the
service, so it stays correct as the crate widens past delivery.

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

Five resources, backed by the observability database.

| Method | Path | |
|---|---|---|
| `GET` | `/alerts/config/definitions` | every definition |
| `POST` | `/alerts/config/definitions` | create one |
| `GET` | `/alerts/config/definitions/{id}` | read one |
| `POST` | `/alerts/config/definitions/{id}` | change part of one |
| `GET` | `/alerts/config/enablement` | every enablement row |
| `GET` | `/alerts/config/enablement/{name}/{product}` | read one |
| `POST` | `/alerts/config/enablement/{name}/{product}` | upsert one |
| `GET` | `/alerts/config/merchant-thresholds` | every merchant threshold, or those matching the query |
| `POST` | `/alerts/config/merchant-thresholds` | upsert one |
| `GET` | `/alerts/config/merchant-thresholds/{id}` | read one |
| `POST` | `/alerts/config/merchant-thresholds/{id}` | change part of one |
| `DELETE` | `/alerts/config/merchant-thresholds/{id}` | delete one |
| `GET` | `/alerts/config/mappers` | every live mapper entry |
| `POST` | `/alerts/config/mappers` | save a new version of one |
| `GET` | `/alerts/config/mappers/{name}/{key}` | read the live one |
| `DELETE` | `/alerts/config/mappers/{name}/{key}` | delete the live one |
| `GET` | `/alerts/config/notifications/read` | read the caller's watermark |
| `POST` | `/alerts/config/notifications/read` | mark read |

A list answers `{"count": n, "<resource>": [...]}` under `definitions`, `enablements`,
`merchant_thresholds` or `entries`, and a table with no rows is a `200` with a count of zero. A
read, create, save, upsert or update answers the row. A delete answers `{"id": "…", "deleted": true}`,
or `{"name": "…", "key": "…", "deleted": true}` for a mapper entry.

An update (`POST` to a definition or merchant threshold id) changes only what the body mentions: an
absent field is left alone, `null` clears it, and a value sets it. `null` for `is_enabled`, or for a
merchant threshold's `author`, leaves it alone. The two upserts are described with their resources.

#### Definitions

A definition is one `alerts_info` row, with an id the service generates. `name`, `product`,
`is_enabled` and `author` are required on create; `name`, `product` and `author` must not be blank,
and `name` and `product` cannot be changed afterwards.

`blacklist`, `snooze` and `thresholds` hold r-apps' documents and are stored exactly as sent, except
that an empty list, an empty object or a blank string is stored as `{}`, as r-apps' `createAlertInfo`
does, and `null` is stored as `NULL`. Each is checked for its shape first:

- `blacklist` is a list, or an object, of groups; a group maps one or more dimensions to a value or
  a list of values, as in `{"ignored_paths": {"path": ["/health", "/ecr"]}}` or
  `[{"merchant_id": "merchant_1234", "payment_method": ["card", "upi"]}]`.
- `thresholds` is one object for the definition, as in
  `{"min_volume": 100, "tolerance": 0.9, "prop_thresholds": {"C030KGG9ZJ9": 0.2}}`.
- `snooze` is an object keyed `snooze_entry_<time>` or `custom_snooze_entry_<time>`; each entry
  carries the dimension values it covers and `snooze_end_time`, optionally `snooze_start_time`, as
  `YYYY-MM-DD HH:MM:SS`.

`metadata` is free-form JSON, except that an empty list, an empty object or a blank string is
stored as `{}`, as `createAlertInfo` does for it. `comments` is free-form JSON, stored as sent.

```http
POST /alerts/config/definitions
{ "name": "sr_drop", "product": "payments", "is_enabled": true, "author": "reliability_team",
  "blacklist": {"test_merchants": {"merchant_id": ["merchant_1234"]}} }
→ 200 { "id": "0199…", "name": "sr_drop", "is_enabled": true, "blacklist": {"test_merchants": …}, … }

POST /alerts/config/definitions/0199…
{ "thresholds": {"min_volume": 100, "tolerance": 2.5} }
→ 200 the whole definition, with blacklist and snooze untouched
```

`all` names the definition for suppression that applies to every detector; it cannot have an
enablement row. There is no delete route for definitions.

#### Enablement

`alerts_info.is_enabled` says whether a detector runs, and `merchants_alert_external_config.is_enabled`
whether its alerts are delivered to merchants. A response carries the stored `is_enabled` and

```
effective_is_enabled = definition.is_enabled AND coalesce(enablement.is_enabled, false)
```

The upsert is one statement with `(name, product)` as its conflict target, and requires
`is_enabled` as a boolean. When the row exists, `category` and `metadata` change only if the body
mentions them, and `null` clears them. It is refused unless a definition with that name and product
exists and the name is not `all`.

```http
POST /alerts/config/enablement/sr_drop/payments
{ "is_enabled": true }
→ 200 { "name": "sr_drop", "is_enabled": true, "effective_is_enabled": false, … }
```

#### Merchant thresholds

A merchant threshold is one `merchant_thresholds` row, r-apps' per-merchant override: `name`,
`product`, `merchant_id`, `author` and `is_enabled`, all required, `metadata`, and ten nullable
numbers `thresholds_min_volume`, `thresholds_min_impacted_volume`, `thresholds_tolerance`,
`thresholds_diff_threshold`, `thresholds_merchant_impact`, `thresholds_alert_period`,
`thresholds_min_observations`, `thresholds_min_history_volume`, `thresholds_filter_percentile` and
`thresholds_current_min_volume`. `name`, `product`, `merchant_id` and `author` must not be blank.

`GET /alerts/config/merchant-thresholds` takes `name`, `product`, `merchant_id`, `is_enabled` and
`author` as query parameters, each an exact match; with none it lists every row. r-apps'
`getMerchantThresholds` filters on every column, with lists of values, ranges and metadata keys; this
service supports only these five exact-match filters.

`POST /alerts/config/merchant-thresholds` upserts on `(name, product, merchant_id, is_enabled,
author)`, the key r-apps' `addMerchantThresholds` uses, so the same five values update one row and
any other combination adds a row. As in r-apps, a threshold or `metadata` that is absent or `null`
keeps the stored value when the row exists and is stored as `NULL` when the row is added. `metadata`,
when given, must be an object and replaces the stored one.

`POST /alerts/config/merchant-thresholds/{id}` changes the thresholds, `metadata`, `author` and
`is_enabled`; `name`, `product` and `merchant_id` cannot be changed. `metadata` must be an object and
is merged into the stored one with `COALESCE(metadata, '{}') || patch`, so a row without metadata
takes the patch; `null` clears it. This differs from r-apps, whose `metadata || patch` leaves a
`NULL` metadata `NULL` and drops the patch.

```http
POST /alerts/config/merchant-thresholds
{ "name": "sr_drop", "product": "payments", "merchant_id": "merchant_1234",
  "author": "reliability_team", "is_enabled": true, "thresholds_tolerance": 2.5 }
→ 200 { "id": "0199…", "merchant_id": "merchant_1234", "thresholds_tolerance": 2.5, "thresholds_min_volume": null, … }

DELETE /alerts/config/merchant-thresholds/0199…
→ 200 { "id": "0199…", "deleted": true }
```

#### Mappers

The option lists and labels behind the portal's mappers screen, stored in `alerts_dicts` and
addressed by `name` and `key`.

```http
POST /alerts/config/mappers
X-Internal-Api-Key: <key>
X-User-Name: ops@example.com

{ "name": "dashboard", "key": "slack_users", "values": "[]", "metadata": {"category": "dashboard"} }
→ 200 { "id": "0192…", "name": "dashboard", "key": "slack_users", "product": null, "values": "[]",
        "metadata": {"category": "dashboard"}, "ts_created": "2026-09-14T12:34:56.789Z",
        "username": "ops@example.com" }

GET    /alerts/config/mappers                        → 200 { "count": 1, "entries": [ … ] }
GET    /alerts/config/mappers/dashboard/slack_users  → 200 the live entry
GET    /alerts/config/mappers/dashboard/unknown      → 404 HE_02
DELETE /alerts/config/mappers/dashboard/slack_users  → 200 { "name": "dashboard", "key": "slack_users", "deleted": true }
DELETE /alerts/config/mappers/dashboard/unknown      → 404 HE_02
```

**A save writes a new version, as r-apps' `insert_dictionary_version` does.** One transaction sets
`is_enabled = false` on the live row, inserts the new row as the live one, and deletes every
disabled row for the same `name` and `key` except the newest, so an entry keeps at most two rows:
the live one and the newest disabled one. The transaction first takes a Postgres advisory lock on
the `name` and `key`, so overlapping saves of one entry run one after the other and the last to
commit is live. Reads and the list return live rows only.

**A delete removes the live row**, as r-apps' `dropDictionary` removes the row it names. It takes
the same lock, so a delete and a save of one entry run one after the other and the delete answers
for the live row as it stands once it holds the lock. The disabled row stays disabled and is not
brought back; the next save writes a new live row.

`product`, `values` and `metadata` are `json` columns carried as raw text in both directions
(`diesel_models::observability::raw_json`), so the portal reads back exactly what it saved. Together
they are capped at 1 MiB.

#### Notification watermark

```http
POST /alerts/config/notifications/read
X-Internal-Api-Key: <key>
X-User-Name: ops@example.com

→ 200 { "last_read_at": "2026-09-14T12:34:56.789Z" }

GET /alerts/config/notifications/read  → 200 { "last_read_at": "2026-09-14T12:34:56.789Z" }
GET /alerts/config/notifications/read  → 404 HE_02 for a user who has never marked read
```

The write takes no body and stamps this service's clock. It stores the later of the saved and the
new instant, so a watermark never moves backwards. A user who has never marked read has no
watermark, and reading it is a `404` like any other missing resource.

#### Who a request is for

The internal API key authenticates the calling service, not a person. A mapper save and both
watermark routes take the user from `X-User-Name`, which nothing authenticates. The header is
required on the watermark routes; on a mapper save it is optional, and when it is absent or blank
`username` takes the column default.

A user name must be visible ASCII and at most 64 characters, the width of
`alerts_dicts.username`. It is held as a `Secret`, so logs and error reports show it masked.

#### Errors

Blank values, widths, document shapes, `X-User-Name` and the mapper entry size are checked before a
database connection is taken. A configuration body that does not parse, including one missing a
required field or giving it `null`, is the `IR_04` above. A path id that is not a UUID is answered with the same empty `404` as a path
that matches no route. The configuration errors, added to the table above:

| | Status | Code |
|---|---|---|
| A field is longer than its column holds, or a name, product, merchant id, author or mapper key is blank | 400 | `IR_07` |
| `X-User-Name` is absent or blank on a watermark route | 400 | `IR_04` |
| `X-User-Name` is not visible ASCII | 400 | `IR_06` |
| `X-User-Name` is longer than 64 characters | 400 | `IR_07` |
| `blacklist`, `snooze` or `thresholds` is not in r-apps' shape, or a merchant threshold's `metadata` is not an object | 400 | `IR_06` |
| The merchant thresholds query string does not parse or names another parameter | 400 | `IR_06` |
| A definition already exists for this name and product | 400 | `HE_01` |
| An update gives a merchant threshold the name, product, merchant, author and `is_enabled` of another | 400 | `HE_01` |
| Name and product do not identify an alert, or name `all` | 400 | `HE_03` |
| A mapper entry's `product`, `values` and `metadata` together exceed 1 MiB | 400 | `HE_03` |
| Unknown definition id, enablement key or merchant threshold id | 404 | `HE_02` |
| No live mapper entry for the name and key, or no watermark for the user | 404 | `HE_02` |
| A query against the observability database failed | 500 | `HE_00` |
| No connection to the observability database could be taken | 503 | `HE_00` |

The failing host, database and role reach the log and never the response.

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
routes/          the route tree, and one module of handlers per area: notify, config, mappers and notifications
core/            what one request does, per area: deliver a message, or read and write configuration
domain/          what delivering an alert is: the notifier traits and the types they exchange
types/           the wire contract, per area
```

Configuration requests are handled in `routes/config.rs` and `core/config.rs`, with their request
and response types and validation in `types/config.rs`; mapper and watermark requests in the
`mappers.rs` and `notifications.rs` of the same three modules, with `X-User-Name` read in `auth.rs`.
The rows are `alerts_info`, `merchants_alert_external_config`, `merchant_thresholds`, `alerts_dicts`
and `notification_reads` in `diesel_models::observability`, their queries are in
`diesel_models::query::observability`, and the tables are created by `migrations/`.

`domain` holds no HTTP. `core` holds no traits. A handler that grows logic belongs in `core`; a
concept that a background job would also need belongs in `domain`.
