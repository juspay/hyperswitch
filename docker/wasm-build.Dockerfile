FROM rust:latest as builder

ARG FEATURES=""
ARG VERSION_FEATURE_SET="v1"

# Install build deps and Binaryen (wasm-opt)
RUN apt-get update \
    && apt-get install -y clang libssl-dev pkg-config curl tar \
    && curl -L https://github.com/WebAssembly/binaryen/releases/download/version_116/binaryen-version_116-x86_64-linux.tar.gz \
    | tar xz \
    && cp binaryen-version_116/bin/wasm-opt /usr/local/bin/

ENV CARGO_INCREMENTAL=0
ENV CARGO_NET_RETRY=10
ENV RUSTUP_MAX_RETRIES=10
ENV RUST_BACKTRACE="short"
ENV env=$env

COPY . .

RUN cargo install wasm-pack

# Disable internal wasm-opt by setting WASM_PACK_OPTIMIZATION to false
ENV WASM_OPT=0

RUN wasm-pack build --target web \
    --out-dir /tmp/wasm \
    --out-name euclid \
    crates/euclid_wasm \
    -- --features ${VERSION_FEATURE_SET},${FEATURES}

# Manually run wasm-opt with correct flags
RUN wasm-opt /tmp/wasm/euclid_bg.wasm \
    -o /tmp/wasm/euclid_bg.wasm \
    --enable-bulk-memory

FROM scratch
COPY --from=builder /tmp/wasm /tmp
