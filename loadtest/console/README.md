# Load-test console

SvelteKit SSR and authenticated APIs manage a single k6 generator. The browser
talks only to the console; the generator calls Router and PM directly.

```bash
cd loadtest/console
npm ci
cp .env.example .env
# Generate a password hash without putting the password in shell history:
read -s "console_password?Admin password: "
printf '%s' "$console_password" | npm run --silent password
unset console_password
# Paste the hash into .env, then configure service credentials.
npm run dev
```

Open http://127.0.0.1:4174. For a local production build:
`npm run build && node --env-file=.env start.mjs`.
The example uses local PostgreSQL on `127.0.0.1:5432`. Configure a database user
with SELECT access to `payment_intent` and its password. Saved Settings override
the initial environment values, so update the saved connection when switching
between local services and a container deployment.

The first start imports the existing ignored `loadtest.config.json` into `.data`;
subsequent settings edits affect the console copy. Set `LOADTEST_CONFIG_FILE` to
import a different configuration and `LOADTEST_DATA_DIR` for durable storage.
Existing artifact directories remain readable. Settings never return stored
passwords, API keys or Grafana tokens. Leaving a secret input blank keeps it.

Create fixtures, then start the load. Flow weights must total 100%. Rate means
payment flows per second; each flow may issue several API attempts. The console
shows successful flows separately from iterations, HTTP error rate and dropped
iterations. Live confirm p90 covers the latest one-second window, capped at
8,192 observations per flow; the final p90 comes from k6's complete summary.
Cards show time-series sparklines from the latest 30 minutes of one-second
telemetry, which is retained with each new run. Older runs without telemetry
show their recorded values without invented trends. Output follows new lines;
scroll up to pause, then select **Follow output** to return to the latest line.

Results offers paginated response history and PostgreSQL reconciliation without
CSV input. Comparison uses the merchant/payment pair, excludes baseline payments,
and requires complete recording unless explicitly overridden. It compares the
latest client status to the current database status; later asynchronous state
changes can therefore appear as mismatches. Full comparisons live in
`reconciliation.sqlite`; `reconciliation.json` retains counts and the first 200
discrepancies. Failed creates with no returned payment ID remain in response
history and cannot be reconciled by ID.

Grafana panel URLs must use the configured server and `/grafana/d-solo/...`.
The iframe uses the authenticated console origin and the selected run's time
window. See [EC2 deployment](DEPLOYMENT.md) for Grafana and ALB configuration.

```bash
npm run check
npm run build
npm test
```

To exercise read-only reconciliation against an existing PostgreSQL database with
at least two payments, supply `PGHOST`, `PGPORT`, `PGDATABASE`, `PGUSER`, and
`PGPASSWORD`, then run `CONSOLE_TEST_PG=1 node --test test/postgres.test.mjs`.
See [validation results](VALIDATION.md) for measurements and remaining EC2 checks.
