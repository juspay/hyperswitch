FROM rust:latest as builder

ARG FEATURES=""
ARG VERSION_FEATURE_SET="v1"

# Install necessary build dependencies and Binaryen (for wasm-opt)
RUN apt-get update \
    && apt-get install -y clang libssl-dev pkg-config curl tar

# Set up environment variables for Rust and build
ENV CARGO_INCREMENTAL=0
ENV CARGO_NET_RETRY=10
ENV RUSTUP_MAX_RETRIES=10
ENV RUST_BACKTRACE="short"

# Copy your Rust project
COPY . .

# Install wasm-pack for initial build
RUN cargo install wasm-pack

# Step 1: Build the WASM binary without running wasm-opt
RUN wasm-pack build --target web \
    --out-dir /tmp/wasm \
    --out-name euclid \
    crates/euclid_wasm \
    -- --features ${VERSION_FEATURE_SET},${FEATURES}

# Step 2: Manually run wasm-opt with the --enable-bulk-memory flag
RUN curl -L https://github.com/WebAssembly/binaryen/releases/download/version_116/binaryen-version_116-x86_64-linux.tar.gz \
    | tar xz -C /tmp && \
    mv /tmp/binaryen-version_116 /tmp/binaryen

# Use the wasm-opt manually to optimize the wasm
RUN /tmp/binaryen/bin/wasm-opt /tmp/wasm/euclid_bg.wasm \
    -o /tmp/wasm/euclid_bg.wasm \
    --enable-bulk-memory

# Final Stage: Copy optimized wasm into the final image
FROM scratch
COPY --from=builder /tmp/wasm /tmp
