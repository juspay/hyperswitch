# Console load engine

This directory contains the k6 engine used by the [load-test console](../../console/README.md).
The console owns configuration, fixture compatibility, scheduling, fault recovery,
recording and reconciliation. No legacy runner or deployment scripts are required.

Build the native recorder with Go 1.24.4 and a C compiler:

```sh
bash recorder/build.sh
```

The engine exports `fixtures` and `load` from `run.mjs`. It calls the configured
Router and payment-method service directly, records responses through the native
SQLite extension, and emits progress for the console and optional remote worker.
`superposition.mjs` applies and verifies fixture defaults; `reconcile.mjs` supports
CSV reconciliation, while the console compares against PostgreSQL directly.

Run engine tests (from this directory):

```sh
K6_SQLITE_BINARY="$PWD/bin/k6-sqlite" node --test test/*.test.mjs
```

Keep generated binaries, merchant credentials, recordings and runtime configuration
out of version control. See `.gitignore` and the console documentation.
