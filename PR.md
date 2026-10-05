## Type of Change

- [x] Bugfix
- [ ] New feature
- [ ] Enhancement
- [ ] Refactoring
- [ ] Dependency updates
- [x] Documentation
- [ ] CI/CD

## Description

Correct the PCR retry-ladder test comment so it agrees with the assertions: 15 total attempts (the initial charge plus 14 retries), with the billing connector owning its first retry and the day-30 retry, leaving 13 retries in the service's ladder.

## Motivation and Context

Fixes [#13972](https://github.com/juspay/hyperswitch/issues/13972), whose comment overstated both the total retry budget and the number of retries covered by the ladder. No runtime behavior changes.

## How did you test it?

Not run yet. This change only updates a test comment; the local environment currently has no Rust toolchain installed.

## Checklist

- [ ] I formatted the code `cargo +nightly fmt --all`
- [ ] I addressed lints thrown by `cargo clippy`
- [x] I reviewed the submitted code
- [ ] I added unit tests for my changes where possible
