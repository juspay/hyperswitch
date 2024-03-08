FROM rust:latest AS builder

WORKDIR /app
COPY . .
RUN cargo install diesel_cli && cargo build --bin router --release

FROM rust:latest AS runtime
WORKDIR /app
COPY --from=builder /app/migrations migrations
COPY --from=builder /app/target/release/router router
COPY --from=builder /usr/local/cargo/bin/diesel diesel
