FROM rust:latest as builder

ARG FEATURES=""
ARG VERSION_FEATURE_SET="v1"

# Install build dependencies and Binaryen (for wasm-opt)
RUN apt-get update \
    && apt-get install -y clang libssl-dev pkg-config curl tar \
    && curl -L https://github.com/WebAssembly/binaryen/releases/download/version_116/binaryen-version_116-x86_64-linux.tar.gz \
    | tar xz \
    && cp binaryen-version_116/bin/wasm-opt /usr/local/bin/wasm-opt

ENV CARGO_INCREMENTAL=0
ENV CARGO_NET_RETRY=10
ENV RUSTUP_MAX_RETRIES=10
ENV RUST_BACKTRACE="short"
ENV env=$env

COPY . .

RUN echo env

# Build the WASM using wasm-pack
RUN cargo install wasm-pack && \
    wasm-pack build --target web \
    --out-dir /tmp/wasm \
    --out-name euclid \
    crates/euclid_wasm \
    -- --features ${VERSION_FEATURE_SET},${FEATURES}

# Optimize with wasm-opt
RUN wasm-opt /tmp/wasm/euclid_bg.wasm \
    -o /tmp/wasm/euclid_bg.wasm \
    --enable-bulk-memory

FROM scratch
COPY --from=builder /tmp/wasm /tmp

