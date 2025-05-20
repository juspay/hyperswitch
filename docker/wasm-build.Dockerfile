FROM rust:latest as builder

ARG FEATURES=""
ARG VERSION_FEATURE_SET="v1"

# Install system dependencies
RUN apt-get update && \
    apt-get install -y curl clang libssl-dev pkg-config tar

# Download and extract wasm-opt separately (DO NOT put in PATH yet)
RUN mkdir -p /opt/binaryen && \
    curl -L https://github.com/WebAssembly/binaryen/releases/download/version_116/binaryen-version_116-x86_64-linux.tar.gz \
    | tar xz -C /opt/binaryen && \
    mv /opt/binaryen/binaryen-version_116 /opt/binaryen/binaryen

# Install wasm-pack
RUN cargo install wasm-pack

COPY . .

ENV CARGO_INCREMENTAL=0
ENV CARGO_NET_RETRY=10
ENV RUSTUP_MAX_RETRIES=10
ENV RUST_BACKTRACE=short

# This prevents wasm-pack / wasm-bindgen from running wasm-opt
ENV WASM_OPT=0

# Build without optimization
RUN wasm-pack build \
    --target web \
    --out-dir /tmp/wasm \
    --out-name euclid \
    crates/euclid_wasm \
    -- --features ${VERSION_FEATURE_SET},${FEATURES}

# Manually run wasm-opt with correct flag
RUN /opt/binaryen/binaryen/bin/wasm-opt \
    /tmp/wasm/euclid_bg.wasm \
    -o /tmp/wasm/euclid_bg.wasm \
    --enable-bulk-memory

# Final stage: ship only artifacts
FROM scratch
COPY --from=builder /tmp/wasm /tmp
