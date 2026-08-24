FROM rust:slim-trixie AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends libpq-dev libssl-dev pkg-config git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /cripta
COPY . .
RUN cargo build --release

FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata libpq-dev curl \
    && rm -rf /var/lib/apt/lists/*

ENV CONFIG_DIR=/local/config \
    BINARY=cripta

COPY --from=builder /cripta/target/release/cripta /local/bin/cripta
WORKDIR /local/bin
EXPOSE 5000
CMD ["./cripta"]
