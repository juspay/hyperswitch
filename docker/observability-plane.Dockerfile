FROM public.ecr.aws/docker/library/rust:trixie AS builder

ARG VERSION_FEATURE_SET="v1"

# Which cargo profile compiles the binary: `release` (default, production),
# `release-fast` (optimized, no LTO, for development cycles) or `dev`
# (unoptimized). Features are untouched by this choice.
ARG CARGO_BUILD_PROFILE=release

RUN apt-get update \
    && apt-get install -y libpq-dev libssl-dev pkg-config protobuf-compiler

WORKDIR /router

# Incremental compilation adds overhead in ephemeral CI build environments.
ENV CARGO_INCREMENTAL=0 \
    CARGO_NET_RETRY=10 \
    RUSTUP_MAX_RETRIES=10 \
    RUST_BACKTRACE="short"

COPY . .
RUN cargo build \
    --package observability \
    --profile ${CARGO_BUILD_PROFILE} \
    --no-default-features \
    --features release \
    --features ${VERSION_FEATURE_SET}

# Cargo places the `dev` profile under `target/debug`.
RUN mkdir -p /router/out \
    && cp "/router/target/$([ "${CARGO_BUILD_PROFILE}" = "dev" ] && echo debug || echo "${CARGO_BUILD_PROFILE}")/observability" /router/out/observability


FROM public.ecr.aws/docker/library/debian:trixie

ARG CONFIG_DIR=/local/config
ARG BIN_DIR=/local/bin
ARG RUN_ENV=sandbox

RUN apt-get update \
    && apt-get install -y ca-certificates tzdata libpq-dev curl procps

EXPOSE 8080

ENV TZ=Etc/UTC \
    RUN_ENV=${RUN_ENV} \
    CONFIG_DIR=${CONFIG_DIR} \
    RUST_MIN_STACK=6291456

RUN mkdir -p ${BIN_DIR}

COPY --from=builder /router/out/observability ${BIN_DIR}/observability

RUN useradd --user-group --system --no-create-home --no-log-init app
USER app:app

WORKDIR ${BIN_DIR}

CMD ["./observability"]
