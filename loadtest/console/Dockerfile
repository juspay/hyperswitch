FROM golang:1.24.4-bookworm AS recorder
WORKDIR /recorder
COPY loadtest/k6/scenario-mix/recorder/go.mod loadtest/k6/scenario-mix/recorder/go.sum ./
RUN go mod download
COPY loadtest/k6/scenario-mix/recorder/ ./
RUN CGO_ENABLED=1 go build -mod=readonly -trimpath -o /k6-sqlite ./cmd/k6-sqlite

FROM node:24.19.0-bookworm-slim AS web
WORKDIR /app/loadtest/console
COPY loadtest/console/package.json loadtest/console/package-lock.json ./
RUN npm ci
COPY loadtest/console/src ./src
COPY loadtest/console/server ./server
COPY loadtest/console/vite.config.js loadtest/console/svelte.config.js loadtest/console/jsconfig.json ./
RUN npm run build && npm prune --omit=dev

FROM node:24.19.0-bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tini time && rm -rf /var/lib/apt/lists/*
WORKDIR /app/loadtest/console
COPY --from=web /app/loadtest/console/build ./build
COPY --from=web /app/loadtest/console/node_modules ./node_modules
COPY loadtest/console/package.json loadtest/console/start.mjs ./
COPY loadtest/console/server ./server
COPY loadtest/console/scripts ./scripts
COPY loadtest/k6/scenario-mix/*.mjs /app/loadtest/k6/scenario-mix/
COPY loadtest/k6/scenario-mix/scenario-mix.js loadtest/k6/scenario-mix/recording-client.js /app/loadtest/k6/scenario-mix/
COPY loadtest/k6/scenario-mix/config.example.json /app/loadtest/k6/scenario-mix/
COPY loadtest/k6/scenario-mix/test/mock-server.mjs /app/loadtest/k6/scenario-mix/test/
COPY loadtest/k6/scenario-mix/recorder/*.sql /app/loadtest/k6/scenario-mix/recorder/
COPY loadtest/loadtest.config.example.json /app/loadtest/
COPY --from=recorder /k6-sqlite /usr/local/bin/k6-sqlite
RUN mkdir /data && chown node:node /data
USER node
ENV NODE_ENV=production HOST=0.0.0.0 PORT=4174 LOADTEST_DATA_DIR=/data LOADTEST_ROOT=/app/loadtest K6_BINARY=/usr/local/bin/k6-sqlite SHUTDOWN_TIMEOUT=70
EXPOSE 4174
STOPSIGNAL SIGTERM
HEALTHCHECK --interval=30s --timeout=5s CMD node -e "fetch('http://127.0.0.1:4174/health').then(r=>process.exit(r.ok?0:1)).catch(()=>process.exit(1))"
ENTRYPOINT ["/usr/bin/tini","--"]
CMD ["node","start.mjs"]
