# observability

The observability plane for Hyperswitch.

`observability` is the home for alert *delivery*. Deciding what is alert-worthy — thresholds,
detectors, suppression — is **not** done here; alerts arrive already decided and this crate routes
them to a destination.

Its first and currently only concern is the [`notifier`](src/domain/notifier.rs): the component that
receives alert data over a webhook and delivers it to a channel. Further alerting concerns are
expected to live alongside it rather than inside it.

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

`observability` has **no `v1`/`v2` feature flags**. The API version duality is the router's
concern, and this crate stays out of it by not depending on any version-flavoured type. Keep it
that way: adding a dependency on `diesel_models` or `hyperswitch_domain_models` would drag the
feature matrix in with it.

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

Two surfaces under one scope. **Delivery** sends a message; **configuration** reads and writes the
rows that say what an alert is and whether it runs. A route under `/alerts/config` touches the
database and nothing else does.

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

Two resources, backed by the observability database.

| Method | Path | |
|---|---|---|
| `GET` | `/alerts/config/definitions` | every definition |
| `POST` | `/alerts/config/definitions` | create one |
| `GET` | `/alerts/config/definitions/{id}` | read one |
| `POST` | `/alerts/config/definitions/{id}` | change part of one |
| `GET` | `/alerts/config/enablement` | every enablement row |
| `GET` | `/alerts/config/enablement/{name}/{product}` | read one |
| `POST` | `/alerts/config/enablement/{name}/{product}` | upsert one |

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

**There is no delete route.** `is_enabled` is how an alert is turned off; unlike a delete it is
reversible, and deleting an `alerts_info` row cascades to every `alerts_main` row referencing it,
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
| Unknown definition id | 404 | `IR_03` |
| Unknown enablement key | 404 | `IR_06` |
| Observability database unreachable | 503 | `HE_01` |

`503` rather than `500`, for the reason `/health/ready` uses it: the service is fine, and the
condition is expected to clear without anyone touching it. The failing host, database and role
reach the log and never the response.

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

Rows and their queries are not here at all: `alerts_info` and `merchants_alert_external_config` are
modelled in `diesel_models::observability`, alongside every other table this database owns, so the
alert manager and this service read one definition of them rather than two.

`domain` holds no HTTP. `core` holds no traits. A handler that grows logic belongs in `core`; a
concept that a background job would also need belongs in `domain`.
