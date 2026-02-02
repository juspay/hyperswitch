FROM rust:latest as builder

ARG FEATURES=""
ARG VERSION_FEATURE_SET="v1"
ARG ENVIRONMENT="development"

RUN apt-get update \
    && apt-get install -y clang libssl-dev pkg-config

ENV CARGO_INCREMENTAL=0
# Allow more retries for network requests in cargo (downloading crates) and
# rustup (installing toolchains). This should help to reduce flaky CI failures
# from transient network timeouts or other issues.
ENV CARGO_NET_RETRY=10
ENV RUSTUP_MAX_RETRIES=10
# Don't emit giant backtraces in the CI logs.
ENV RUST_BACKTRACE="short"
# Set ENVIRONMENT for build.rs to read SDK URL from config/sdk_urls.toml
ENV ENVIRONMENT=${ENVIRONMENT}

COPY . .

RUN cargo install wasm-pack

# Build payment_link WASM with ENVIRONMENT variable set
# This allows build.rs to select the correct SDK URL from config/sdk_urls.toml
RUN ENVIRONMENT=${ENVIRONMENT} wasm-pack build \
    --target web \
    --out-dir /tmp/wasm \
    --out-name payment_link \
    crates/payment_link \
    -- --features wasm,${VERSION_FEATURE_SET},${FEATURES}

FROM scratch

COPY --from=builder /tmp/wasm /tmp
