# Migration Runner Dockerfile for Hyperswitch
# This image contains migration files and diesel CLI for offline migration execution

FROM debian:bookworm

# Install necessary packages
RUN apt-get update && apt-get install -y \
    curl \
    tar \
    xz-utils \
    ca-certificates \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Install diesel CLI (support multi-arch)
ENV DIESEL_VERSION=2.3.0
RUN ARCH=$(uname -m) && \
    case "${ARCH}" in \
      x86_64) DIESEL_ARCH="x86_64-unknown-linux-gnu" ;; \
      aarch64) DIESEL_ARCH="aarch64-unknown-linux-gnu" ;; \
      *) echo "Unsupported architecture: ${ARCH}" && exit 1 ;; \
    esac && \
    curl -L "https://github.com/diesel-rs/diesel/releases/download/v${DIESEL_VERSION}/diesel_cli-${DIESEL_ARCH}.tar.xz" | tar -xJ -C /tmp/ && \
    mv "/tmp/diesel_cli-${DIESEL_ARCH}/diesel" /usr/local/bin/diesel && \
    rm -rf "/tmp/diesel_cli-${DIESEL_ARCH}" && \
    chmod +x /usr/local/bin/diesel

# Set working directory
WORKDIR /hyperswitch

# Copy migration files and diesel config from the workspace
COPY ./migrations/ ./migrations/
COPY ./diesel.toml ./diesel.toml

# Create the 'app' user and group (following hyperswitch pattern)
RUN useradd --user-group --system --no-create-home --no-log-init app

# Change ownership of working directory to app user
RUN chown -R app:app /hyperswitch

USER app:app

# Default command to run migrations
# Note: DATABASE_URL environment variable should be provided at runtime
CMD ["diesel", "migration", "run"]
