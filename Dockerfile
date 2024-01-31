FROM rust:bookworm as builder

ARG EXTRA_FEATURES=""

RUN apt-get update \
    && apt-get install -y libpq-dev libssl-dev pkg-config

# Copying codebase from current dir to /router dir
# and creating a fresh build
WORKDIR /router

# Disable incremental compilation.
#
# Incremental compilation is useful as part of an edit-build-test-edit cycle,
# as it lets the compiler avoid recompiling code that hasn't changed. However,
# on CI, we're not making small edits; we're almost always building the entire
# project from scratch. Thus, incremental compilation on CI actually
# introduces *additional* overhead to support making future builds
# faster...but no future builds will ever occur in any given CI environment.
#
# See https://matklad.github.io/2021/09/04/fast-rust-builds.html#ci-workflow
# for details.
ENV CARGO_INCREMENTAL=0
# Allow more retries for network requests in cargo (downloading crates) and
# rustup (installing toolchains). This should help to reduce flaky CI failures
# from transient network timeouts or other issues.
ENV CARGO_NET_RETRY=10
ENV RUSTUP_MAX_RETRIES=10
# Don't emit giant backtraces in the CI logs.
ENV RUST_BACKTRACE="short"
# Use cargo's sparse index protocol
ENV CARGO_REGISTRIES_CRATES_IO_PROTOCOL="sparse"

COPY . .
RUN cargo build --release --features release ${EXTRA_FEATURES}



FROM debian:bookworm

# Placing config and binary executable in different directories
ARG CONFIG_DIR=/local/config
ARG BIN_DIR=/local/bin

# RUN_ENV decides the corresponding config file to be used
ARG RUN_ENV=sandbox

# args for deciding the executable to export. three binaries:
# 1. BINARY=router - for main application
# 2. BINARY=scheduler, SCHEDULER_FLOW=consumer - part of process tracker
# 3. BINARY=scheduler, SCHEDULER_FLOW=producer - part of process tracker
ARG BINARY=router
ARG SCHEDULER_FLOW=consumer

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata libpq-dev curl procps

EXPOSE 8080

ENV TZ=Etc/UTC \
    RUN_ENV=${RUN_ENV} \
    CONFIG_DIR=${CONFIG_DIR} \
    SCHEDULER_FLOW=${SCHEDULER_FLOW} \
    BINARY=${BINARY}

RUN mkdir -p ${BIN_DIR}

COPY --from=builder /router/target/release/${BINARY} ${BIN_DIR}/${BINARY}

WORKDIR ${BIN_DIR}

CMD ./${BINARY}
