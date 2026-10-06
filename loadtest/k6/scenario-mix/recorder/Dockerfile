FROM golang:1.24.4-bookworm AS build
WORKDIR /recorder
COPY go.mod go.sum ./
RUN go mod download
COPY . .
RUN CGO_ENABLED=1 go build -mod=readonly -trimpath -o /k6-sqlite ./cmd/k6-sqlite
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /k6-sqlite /usr/local/bin/k6-sqlite
ENTRYPOINT ["k6-sqlite"]
