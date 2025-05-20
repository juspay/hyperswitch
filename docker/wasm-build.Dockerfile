FROM rust:latest as builder

ARG FEATURES=""
ARG VERSION_FEATURE_SET="v1"

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
ENV env=$env
COPY . .
RUN echo env
RUN cargo install wasm-opt
RUN cargo install wasm-pack
RUN wasm-pack build --target web --out-dir /tmp/wasm --out-name euclid crates/euclid_wasm -- --features ${VERSION_FEATURE_SET},${FEATURES}
RUN wasm-opt /tmp/wasm/euclid_bg.wasm -o /tmp/wasm/euclid_bg.wasm \
    --enable-bulk-memory \
    --enable-reference-types \
    --enable-simd \
    --enable-threads \
    --enable-mutable-globals \
    --enable-nontrapping-float-to-int \
    -O3
FROM scratch

COPY --from=builder /tmp/wasm /tmp