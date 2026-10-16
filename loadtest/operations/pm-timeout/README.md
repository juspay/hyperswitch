# PM outgoing HTTP timeout change

Prepared against `~/codes/hyperswitch` tag `2026.10.02.0.fix1`, matching the deployed PM release lineage. `timeout.patch` adds two optional settings:

- `ROUTER__PROXY__CONNECT_TIMEOUT=15`: bounds DNS/TCP/TLS connection establishment to 15 seconds.
- `ROUTER__PROXY__REQUEST_TIMEOUT=15`: sets the default total HTTP request timeout to 15 seconds, including connection and response processing. Explicit per-call timeouts retain their existing precedence.

Both settings default to absent, preserving current behavior: no separate connect deadline and a 30-second default total request timeout. The idle pool timeout is unrelated. Configure these only on `payment-method-modular-alloydb-hyperswitch-server` in namespace `hyperswitch`.

The deployment cannot use these settings until a new PM image is built. Merely setting environment variables on the old image will not enforce either timeout.

Prepared source is in `/private/tmp/pm-timeout-20261010`. Build/publication to Cloud Build project `hyperswitch-dev` and Artifact Registry repository `asia-south1-docker.pkg.dev/hyperswitch-dev/hyperswitch-modular-pm` requires the requested approval. No source has been uploaded and no image or cloud deployment has been changed.

After approval: build the PM v2 image, set the selected timeout environment variables on PM alone, update its image, wait for its one replica to be Ready, verify `/health`, and test a stalled vault connection/response against the selected deadline. Preserve the router image and environment. Keep the previous PM image as the rollback target.
