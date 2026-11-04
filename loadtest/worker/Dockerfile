FROM golang:1.24.4-bookworm AS recorder
WORKDIR /recorder
COPY loadtest/k6/scenario-mix/recorder/go.mod loadtest/k6/scenario-mix/recorder/go.sum ./
RUN go mod download
COPY loadtest/k6/scenario-mix/recorder/ ./
RUN CGO_ENABLED=1 go build -mod=readonly -trimpath -o /k6-sqlite ./cmd/k6-sqlite

FROM node:24.19.0-bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tini && rm -rf /var/lib/apt/lists/*
WORKDIR /opt/hyperswitch/loadtest
COPY loadtest/worker ./worker
COPY loadtest/k6 ./k6
COPY --from=recorder /k6-sqlite /usr/local/bin/k6-sqlite
RUN useradd --create-home worker && mkdir /data && chown worker:worker /data
USER worker
ENV LOADTEST_ROOT=/opt/hyperswitch/loadtest LOADTEST_WORKER_DATA_DIR=/data K6_BINARY=/usr/local/bin/k6-sqlite
ENTRYPOINT ["/usr/bin/tini","--","node","worker/agent.mjs"]
