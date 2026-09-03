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

Two routes, one per channel. **The path says where, the body says what** — the URL names the
channel and the destination, the body carries only content. Channel ids, recipient addresses and
credentials live in configuration, so a caller cannot address a channel that was not set up for it
and no credential travels on the wire.

The whole surface, guarded and not:

| Method | Path | Auth |
|---|---|---|
| `POST` | `/alerts/chat/notify/{destination}` | `X-Internal-Api-Key` |
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
routes/   the route tree and handlers; deserialize, call core, serialize
core/     what one request does: resolve a destination, hand the message over
domain/   what delivering an alert is: the notifier traits and the types they exchange
```

`domain` holds no HTTP. `core` holds no traits. A handler that grows logic belongs in `core`; a
concept that a background job would also need belongs in `domain`.
