FROM rust:slim as builder

ARG RUN_ENV=development
WORKDIR /bench

COPY . .
WORKDIR /bench/crates/benchmarker


RUN apt-get update \
    && apt-get install -y libpq-dev libssl-dev pkg-config

RUN cargo bench --no-run

CMD [ "cargo", "bench" ] 
