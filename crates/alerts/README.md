# alerts

The alerting plane for Hyperswitch.

`alerts` is the home for alert *delivery*. Deciding what is alert-worthy — thresholds, detectors,
suppression — is **not** done here; alerts arrive already decided and this crate routes them to a
destination.

Its first and currently only concern is the [`notifier`](src/core/notifier.rs): the component that
receives alert data over a webhook and delivers it to a channel. Further alerting concerns are
expected to live alongside it rather than inside it.

## Shape

The crate ships two ways, on the `drainer` model:

- **Standalone** — `cargo run -p alerts`, its own `actix` `HttpServer`, released independently.
- **Embedded** — a library exposing an `actix` `Scope` the router can mount in-process.

Only the standalone path is wired up today. Mounting in the router is deliberately deferred; the
crate exposes `Scope` factories rather than raw handlers so both paths share one route definition
when that lands.

## Versioning

`alerts` has **no `v1`/`v2` feature flags**. The API version duality is the router's concern, and
this crate stays out of it by not depending on any version-flavoured type. Keep it that way:
adding a dependency on `diesel_models` or `hyperswitch_domain_models` would drag the feature
matrix in with it.

## Configuration

Reads `config/alerts.toml` by default; override with `-f <path>`. Every value can be overridden by
an environment variable prefixed `ALERTS__`, with `__` separating levels — so
`ALERTS__AUTH__INTERNAL_API_KEY` sets `auth.internal_api_key`.

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
trading typed extraction for the concealment of a public schema. `tests/notify.rs` asserts it so it
stays a recorded property.

## The API

Two routes, one per channel. The path names the channel; the body names a **destination id**, and
nothing else. Channel ids, recipient addresses and credentials live in configuration, so a caller
cannot address a channel that was not set up for it and no credential travels on the wire.

```http
POST /alerts/chat/notify
X-Internal-Api-Key: <key>

{ "destination": "sr_alerts", "text": "*3 merchants not converting*", "reply_to": "1503435956.000247" }
→ 200 { "message_id": "1503435991.000318" }
```

```http
POST /alerts/email/notify
X-Internal-Api-Key: <key>

{ "destination": "oncall", "subject": "[Hyperswitch] 3 merchants not converting", "body": "<pre>…</pre>" }
→ 200 {}
```

`reply_to` takes a `message_id` from an earlier response and threads under it. It exists on chat
only; sending it to `/email/notify` is a `400`, not a silently dropped field, because a recovery
notice that quietly loses its link to the alert it clears is a bug nobody notices.

**Nothing here renders.** `text`, `subject` and `body` are delivered exactly as they arrive, so the
caller owns markup and escaping. That is deliberate: the `hyperswitch-alerts` R service already
renders a summary in Slack `mrkdwn` for chat and a full per-alert list for email, and they are not
the same message. `body` is **HTML**, because both email backends in `external_services` hardcode
an HTML body and there is no plain-text path to reach.

An alternative shape was considered and rejected: one `POST /notify/{id}` over a channel-tagged
body. The id already resolves the channel through configuration, so a tag in the body is a second
authority on the same fact, they can disagree, and every implementation ends up carrying a match
arm for a channel it can never serve.

### Errors

Split by **blame**, not by which layer raised them. The provider reports an oversized message and
an unknown channel through the same field of the same response, and they are not the same problem:
one the caller can fix, one it cannot. `reason` carries the provider's own code so a caller can
match on it rather than parse prose.

| | Status | Code |
|---|---|---|
| Body did not parse | 400 | `IR_04` |
| Unknown destination | 400 | `IR_02` |
| Provider refused, and the caller can fix it (`msg_too_long`, `thread_not_found`) | 400 | `IR_03` |
| Missing or wrong API key | 401 | `IR_01` |
| Provider is rate limiting us | 429 | `HE_04`, plus a `Retry-After` header |
| Provider refused because of our configuration (`channel_not_found`, `invalid_auth`) | 500 | `HE_02` |
| Provider unreachable, or its answer was unreadable | 502 | `HE_03` |

## Destinations

Configured under `chat.destinations.<id>` and `email.destinations.<id>`, resolved once at boot.

**Ids set from the environment arrive lowercased and cannot contain `__`.** The `config` crate
lowercases every environment key before splitting it, and `__` is the level separator, so
`ALERTS__CHAT__DESTINATIONS__SR_ALERTS__CHANNEL` sets `chat.destinations.sr_alerts.channel` and
there is no spelling that yields `SR_ALERTS`. The service refuses to start on an id that would not
survive the round trip, rather than failing to match it at lookup time.

A chat destination is tagged: `xyne`, `slack`, or `log`. Xyne and Slack are one client differing in
base URL and credential, not two integrations. `log` accepts messages and delivers nothing, and
exists so the whole path can be exercised before real credentials arrive. It logs sizes and never
the message, since it writes to the same stream as everything else and an alert body carries
merchant ids and payment volumes.

Having **no** destinations is a warning, not a boot failure: a first deployment has none until
credentials exist, and refusing to start would make the service undeployable before them. A
destination that is configured but cannot be built *is* a boot failure, because dropping it would
leave the service answering "unknown destination" to something that is very much configured.

Two limits are inherited from `external_services::email` and tracked in hyperswitch-cloud#23160.
An email destination holds one address, so reaching three people is three destinations; and the
body must be HTML. Email delivery itself is not wired yet — every email destination accepts and
logs until hyperswitch-cloud#23111 lands the transport.
