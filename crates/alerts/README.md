# alerts

The alerting plane for Hyperswitch.

`alerts` is the home for alert *delivery*. Deciding what is alert-worthy — thresholds, detectors,
suppression — is **not** done here; alerts arrive already decided and this crate routes them to a
destination.

Its first and currently only concern is the [`notifier`](src/notifier.rs): the component that
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
