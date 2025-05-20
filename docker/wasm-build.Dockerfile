FROM rust:latest as builder

ARG FEATURES=""
ARG VERSION_FEATURE_SET="v1"

# Install required tools
RUN apt-get update && \
    apt-get install -y clang libssl-dev pkg-config curl tar

ENV CARGO_INCREMENTAL=0
ENV CARGO_NET_RETRY=10
ENV RUSTUP_MAX_RETRIES=10
ENV RUST_BACKTRACE="short"

COPY . .

# Install wasm-pack
RUN cargo install wasm-pack

# 📦 Download wasm-opt separately and store in non-PATH dir
RUN mkdir -p /opt/binaryen && \
    curl -L https://github.com/WebAssembly/binaryen/releases/download/version_116/binaryen-version_116-x86_64-linux.tar.gz \
    | tar xz -C /opt/binaryen && \
    mv /opt/binaryen/binaryen-version_116 /opt/binaryen/binaryen

# 🧨 IMPORTANT: Hide wasm-opt from PATH during wasm-pack build
RUN PATH="/usr/bin:/bin:/usr/local/sbin:/usr/local/bin" \
    wasm-pack build \
    --target web \
    --out-dir /tmp/wasm \
    --out-name euclid \
    crates/euclid_wasm \
    -- --features ${VERSION_FEATURE_SET},${FEATURES}

# ✅ Run wasm-opt manually with --enable-bulk-memory
RUN /opt/binaryen/binaryen/bin/wasm-opt \
    /tmp/wasm/euclid_bg.wasm \
    -o /tmp/wasm/euclid_bg.wasm \
    --enable-bulk-memory

FROM scratch
COPY --from=builder /tmp/wasm /tmp
