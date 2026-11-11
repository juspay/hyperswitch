# Load testing

Docker bundles Node, the UI and the compiled Go/k6 recorder; no host toolchain is
needed. Run Hyperswitch and its required services separately. For services on your
Mac, use `host.docker.internal` rather than `127.0.0.1` in connection settings.

## Loadtesting using CLI

From the repository root, build the shared image and mount a local artifact directory:

```sh
docker build -t hyperswitch-loadtest -f loadtest/console/Dockerfile .
mkdir -p loadtest/artifacts

# Run the engine inside the image; /data persists on the host.
loadtest_cli() {
  docker run --rm -i --user "$(id -u):$(id -g)" \
    -v "$PWD/loadtest/artifacts:/data" hyperswitch-loadtest \
    node /app/loadtest/k6/scenario-mix/run.mjs "$@"
}

loadtest_cli fixtures --config /data/provision.json --output /data/fixtures/local
loadtest_cli load --config /data/load.json --fixtures /data/fixtures/local \
  --output /data/runs/test-1 --k6 /usr/local/bin/k6-sqlite
loadtest_cli reconcile --db /data/runs/test-1/results.sqlite \
  --csv /data/payments.csv --output /data/reconciliation.csv
```

Before running, place your provisioning and load configurations in
`loadtest/artifacts/`, using the settings in
[`loadtest.config.example.json`](loadtest.config.example.json). Configure concrete
credentials or explicitly pass the required environment variables to `docker run`.
Use a fresh output directory for each run. CLI reconciliation uses a payment CSV.
For native execution and tests, see the [engine guide](k6/scenario-mix/README.md).

## Loadtesting using console

```sh
cd loadtest/console
cp .env.example .env
# Set the admin password hash and service credentials; see the setup guide.
docker compose up -d --build
```

Open http://127.0.0.1:4174. Configure **Settings**, create a **Scenario**, provision
**Fixtures**, start load in **Performance**, and inspect **Results**. The console
also manages faults, pause/resume, distributed workers and PostgreSQL reconciliation.

See the [setup guide](console/README.md) for configuration and native development
(Node 24.19+, Go 1.24.4 and a C compiler), the
[screenshots](console/README.md#ui-screenshots) and
[deployment guide](console/DEPLOYMENT.md). Keep credentials, generated fixtures,
recordings and binaries out of version control.
