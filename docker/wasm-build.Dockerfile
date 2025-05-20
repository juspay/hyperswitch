FROM rust:latest as builder

ARG FEATURES=""
ARG VERSION_FEATURE_SET="v1"

# Install build dependencies and wasm-opt (Binaryen)
RUN apt-get update \
    && apt-get install -y clang libssl-dev pkg-config curl \
    && curl -sL https://github.com/WebAssembly/binaryen/releases/download/version_117/binaryen-version_117-x86_64-linux.tar.gz \
    | tar -xz \
    && mv binaryen-version_117/bin/wasm-opt /usr/local/bin/ \
    && chmod +x /usr/local/bin/wasm-opt

ENV CARGO_INCREMENTAL=0 \
    CARGO_NET_RETRY=10 \
    RUSTUP_MAX_RETRIES=10 \
    RUST_BACKTRACE=short

COPY . .

RUN cargo install wasm-pack

# Build without wasm-opt in wasm-pack, we'll run it manually
RUN wasm-pack build \
    --target web \
    --out-dir /tmp/wasm \
    --out-name euclid \
    --no-wasm-opt \
    crates/euclid_wasm \
    -- --features ${VERSION_FEATURE_SET},${FEATURES}

# Optimize the wasm output using wasm-opt with bulk-memory support
RUN wasm-opt /tmp/wasm/euclid_bg.wasm \
    -o /tmp/wasm/euclid_bg.wasm \
    --enable-bulk-memory

# Final minimal image
FROM scratch
COPY --from=builder /tmp/wasm /tmp
