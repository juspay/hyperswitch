# Two-step payment-flow load test

Requires Node 24.19+. From the repository root, use the same configuration for either **single**
(1 merchant) or **tenancy** (1,500 merchants):

```bash
# Once: build the recorded k6 binary and configure credentials.
bash loadtest/k6/scenario-mix/recorder/build.sh
cp loadtest/loadtest.config.example.json loadtest/loadtest.config.json
# Set ROUTER_ADMIN_API_KEY and SUPERPOSITION_TOKEN; edit flags/load settings.

node loadtest/loadtest.mjs fixtures single
node loadtest/loadtest.mjs load single

node loadtest/loadtest.mjs fixtures tenancy
node loadtest/loadtest.mjs load tenancy
```

The config defaults to Router **8080**, PM **8081**, Superposition **8082**, and
**10 payment flows/sec for 60 seconds**. `fixtures` creates merchant accounts,
API keys, profiles and MCAs, applies configured workspace flags, waits for service
refresh, and saves a ready manifest. It resumes when rerun. `load` uses that
manifest, chooses merchants uniformly per iteration, and records every API attempt
to SQLite. Tenancy TPS is the total across 1,500 merchants. Increase
`load.total_rps` and `load.duration_seconds` in the config for larger runs.
Tune VU multipliers to the full-flow latency; long polling or timeouts need more
concurrent VUs. Default thresholds fail a run with dropped iterations or at least
1% HTTP failures, while retaining its recorded responses for investigation.
Load execution shows k6's live progress and prints a reconciliation command for
its database when finished, including failed tests. Fixture setup shows progress
for merchants, flag resolution, service refresh, and final verification. Redirected
fixture output uses periodic progress lines instead of terminal redraws.
The example uses the local `stripe_test` dummy connector; replace its configuration
with your desired connector for external connector testing.

Outputs go into ignored `loadtest/artifacts/local/{single,tenancy}/`; every load
gets a fresh timestamped run directory. Add `--config /path/to/config.json` to
either command to use another config. See
[recording and reconciliation details](k6/scenario-mix/README.md).

The supplied mix is 50% modular CIT off-session, 30% SDK checkout, and 20%
non-modular guest. Workspace defaults enable PM modular calls, select
`pay_then_vault`, and enable fingerprint migration. Changes to these defaults
require rerunning `fixtures` for the selected mode before `load` will accept them.

---

## Performance Benchmarking Setup

The setup uses docker compose to get the required components up and running. It also handles running database migration
and starts [K6 load testing](https://k6.io/docs/) script at the end. The metrics are visible in the console as well as
through Grafana dashboard.

We have added a callback at the end of the script to compare result with existing baseline values. The env variable
`LOADTEST_RUN_NAME` can be used to change the name of the run which will be used to create json, result summary and diff
benchmark files. The default value is "baseline", and diff will be created by comparing new results against baseline.
See 'How to run' section.

###  Structure/Files

`config`:   contains router toml file to change settings. Also setting files for other components like Tempo etc.

`grafana`:  data source and dashboard files

`k6`:       K6 load testing tool scripts. The `setup.js` contain common functions like creating merchant api key etc.
            Each js files will contain load testing scenario of each APIs. Currently, we have `health.js` and `payment-confirm.js`.

`.env`:     It provide default value to docker compose file. Developer can specify which js script they want to run using env
            variable called `LOADTEST_K6_SCRIPT`. The default script is `health.js`. See 'How to run' section.

### How to run

Build image of checked out branch.
```bash
docker compose build
```

Run default (`health.js`) script. It will generate baseline result.
```bash
bash loadtest.sh
```

The `loadtest.sh` script takes following flags,

`-c`: _compare_ with baseline results [without argument]
      auto assign run name based on current commit number

`-r`: takes _run name_ as argument (default: baseline)

`-s`: _script name_ exists in `k6` directory without the file extension as argument (default: health)

`-a`: run loadtest for _all scripts_ existing in `k6` directory [without argument]

For example, to run the baseline for `payment-confirm.js` script.
```bash
bash loadtest.sh -s payment-confirm
```

The run name could be anything. It will be used to prefix benchmarking files, stored at `./k6/benchmark`. For example,
```bash
bash loadtest.sh -r made_calls_asyns -s payment-confirm
```

A preferred way to compare new changes with the baseline is using the `-c` flag. It automatically assigns commit numbers to
easily match different results.
```bash
bash loadtest.sh -c -s payment-confirm
```

Assuming there is baseline files for all the script, following command will compare them with new changes,
```bash
bash loadtest.sh -ca
```
It uses `-c` compare flag and `-a` run loadtest using all the scripts.

Developer can observe live metrics using [K6 Load Testing Dashboard](http://localhost:3002/d/k6/k6-load-testing-results?orgId=1&refresh=5s&from=now-1m&to=now) in Grafana.
The [Tempo datasource](http://localhost:3002/explore?orgId=1&left=%7B%22datasource%22:%22P214B5B846CF3925F%22,%22queries%22:%5B%7B%22refId%22:%22A%22,%22queryType%22:%22nativeSearch%22%7D%5D,%22range%22:%7B%22from%22:%22now-1m%22,%22to%22:%22now%22%7D%7D)
is available to inspect tracing of individual requests.

### Notes

1. The script will first "down" the already running docker compose to run loadtest on freshly created database.
2. Make sure that the Rust compiler is happy with your changes before you start running a performance test. This will save a lot of your time.
3. If the project image is available locally then `docker compose up` won't take your new changes into account.
   Either first do `docker compose build` or `docker compose up --build k6`.
4. For baseline, make sure you in the right branch and have build the image before running the loadtest script.
