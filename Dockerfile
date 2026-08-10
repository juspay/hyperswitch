FROM public.ecr.aws/docker/library/rust:trixie as builder

ARG EXTRA_FEATURES=""
ARG VERSION_FEATURE_SET="v1"

# Which cargo profile compiles the binaries. The default is the production
# build and is deliberately the slowest one: `release` carries fat LTO, a
# single codegen unit and stripped symbols, trading ~an hour of compile time
# for peak runtime speed. The alternatives exist for development cycles,
# where compile time is the resource that matters:
#
#   release-fast — optimized, but without LTO and with parallel codegen:
#                  builds in a fraction of the time, runs modestly slower,
#                  and keeps symbols so backtraces resolve.
#   dev          — unoptimized: the fastest build and a MUCH slower binary.
#                  For wiring and smoke work only, never for measurements —
#                  and note debug frames are deeper, so stack headroom that
#                  suffices in release may not here.
#
# Features are untouched by this choice: every profile builds the same
# feature set, so a fast build differs in codegen only, not in behavior.
ARG CARGO_BUILD_PROFILE=release

RUN apt-get update \
    && apt-get install -y libpq-dev libssl-dev pkg-config protobuf-compiler

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

COPY . .
RUN cargo build \
    --profile ${CARGO_BUILD_PROFILE} \
    --no-default-features \
    --features release \
    --features ${VERSION_FEATURE_SET} \
    ${EXTRA_FEATURES}

# Stage the binary at a profile-independent path for the runtime stage.
# The artifact directory is named after the profile — except `dev`, which
# cargo places under `target/debug` for historical reasons.
#
# BINARY is consumed here and not before the build, so the expensive build
# layer above stays identical across the router/consumer/producer images and
# is shared by the layer cache; only this copy step varies per image.
ARG BINARY=router
RUN mkdir -p /router/out \
    && cp "/router/target/$([ "${CARGO_BUILD_PROFILE}" = "dev" ] && echo debug || echo "${CARGO_BUILD_PROFILE}")/${BINARY}" "/router/out/${BINARY}"



FROM public.ecr.aws/docker/library/debian:trixie

# Placing config and binary executable in different directories
ARG CONFIG_DIR=/local/config
ARG BIN_DIR=/local/bin

# Copy this required fields config file
COPY --from=builder /router/config/payment_required_fields_v2.toml ${CONFIG_DIR}/payment_required_fields_v2.toml

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
    BINARY=${BINARY} \
    RUST_MIN_STACK=6291456

RUN mkdir -p ${BIN_DIR}

COPY --from=builder /router/out/${BINARY} ${BIN_DIR}/${BINARY}

# Create the 'app' user and group
RUN useradd --user-group --system --no-create-home --no-log-init app
USER app:app

WORKDIR ${BIN_DIR}

CMD ./${BINARY}
