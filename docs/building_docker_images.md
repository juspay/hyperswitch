# Building Docker Images

## Cargo Features

The Hyperswitch application server makes extensive use of
[Cargo features][cargo-features] to selectively toggle certain features and
integrations.
The list of features can be found in the `[features]` section in the
[`crates/router/Cargo.toml`][router-manifest] file.

Of these features, the noteworthy ones are the `default` and `release` features:

- `default`: This feature set enables a basic set of necessary features for both
  development and production environments, such as the transactional APIs,
  APIs used by the control center, Stripe compatibility, automatic payment
  retries, in-memory caching, etc.

- `release`: This feature set enables some additional features that are suitable
  for production environments, such as AWS KMS integration, AWS S3 integration,
  AWS SES integration, etc.

Refer to the [documentation on cargo features][cargo-features] to understand how
to select features with Cargo, when building the application server.

## Building with the Dockerfile

The Docker images for the application server and other components can be built
using the [`Dockerfile`][dockerfile] using commands like so, substituting the
Docker image tags with suitable values:

- router:

  ```shell
  docker build \
    --load \
    --file Dockerfile \
    --build-arg "BINARY=router" \
    --tag hyperswitch-router \
    .
  ```

- consumer:

  ```shell
  docker build \
    --load \
    --file Dockerfile \
    --build-arg "BINARY=scheduler" \
    --build-arg "SCHEDULER_FLOW=consumer" \
    --tag hyperswitch-consumer \
    .
  ```

- producer:

  ```shell
  docker build \
    --load \
    --file Dockerfile \
    --build-arg "BINARY=scheduler" \
    --build-arg "SCHEDULER_FLOW=producer" \
    --tag hyperswitch-producer \
    .
  ```

- drainer:

  ```shell
  docker build \
    --load \
    --file Dockerfile \
    --build-arg "BINARY=drainer" \
    --tag hyperswitch-drainer \
    .
  ```

When our Docker images are built using the [`Dockerfile`][dockerfile], the
`cargo` command being run is:

```shell
cargo build --release --features release ${EXTRA_FEATURES}
```

- The `--release` flag specifies that optimized release binaries must be built.
  Refer to the [`cargo build` manual page][cargo-build-manual-page] for more
  information.

- The `--features release` flag specifies that the `release` feature set must be
  enabled for the build.
  Since we do not specify the `--no-default-features` flag to the `cargo build`
  command, the build would have the `default` and `release` features enabled.

- The `${EXTRA_FEATURES}` build argument can specify any additional features
  that would have to be passed to the `cargo build` command.
  The build argument could look like so:
  `EXTRA_FEATURES="--features feature1,feature2,feature3"`, with actual feature
  names substituted in the command.

## Image Variants on Docker Hub

As of writing this document, we have two image variants available on our Docker
Hub repositories:

- release: These images contain only the tag that was built, and no other
  suffixes, like the `v1.105.1` and `v1.107.0` Docker images.

- standalone: These images contain the tag that was built with a `standalone`
  suffix, like the `v1.105.1-standalone` and `v1.107.0-standalone` Docker images.

Our standalone Docker images differ from the release images in that the
standalone images have some features disabled to allow running the application
outside cloud hosted environments like AWS.
As of writing this document, the standalone images exclude the `email` and
`recon` features from the `release` feature set, while the release images are
built from the Dockerfile, without any changes to the codebase after the tag is
checked out.

If you are building custom images and would like to mirror the behavior of our
standalone images, then you'd have to remove the `email` and `recon` features
from the `release` feature set.

## Frequently Asked Questions

### What machine specifications would I need to build release images?

Building release (optimized) images needs significant amount of resources, and
we'd recommend using a machine with at least 8 cores and 16 GB of RAM for this
purpose.
Rust is known to have long compile times, and a codebase of this size will
require a significant time to build, from around 45 minutes to an hour for
release images.

### Build seems to be stuck at "Compiling router/scheduler/analytics/..."

The compilation process involves compiling all of our dependencies and then
compiling our workspace (first-party) crates, among which the biggest one
(in terms of lines of code) is the `router` crate.
Once all the dependencies of the `router` crate have been built, one of the last
ones being built is the `router` crate itself.

As mentioned above, building release images takes a significant amount of time,
so nothing else being printed after a line which says
`Compiling router / scheduler / analytics / ...` is normal, we'd suggest waiting
for a while.
If you're still concerned that the compilation process has been stuck for far
too long, you can check if at least one CPU is being utilized by
`cargo` / `docker` using a tool like `htop` (if you can access the machine which
is building the code), and if not, you can proceed to kill the compilation
process and try again.

[cargo-features]: https://doc.rust-lang.org/cargo/reference/features.html
[router-manifest]: https://github.com/juspay/hyperswitch/blob/main/crates/router/Cargo.toml
[dockerfile]: https://github.com/juspay/hyperswitch/blob/main/Dockerfile
[cargo-build-manual-page]: https://doc.rust-lang.org/cargo/commands/cargo-build.html
