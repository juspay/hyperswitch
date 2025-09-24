# Migration Runner Dockerfile for Hyperswitch
# This image contains migration files and diesel CLI for offline migration execution

FROM debian:trixie-slim

# Install necessary packages
RUN apt-get update && apt-get install -y \
    curl \
    tar \
    xz-utils \
    ca-certificates \
    libpq-dev \
    && rm -rf /var/lib/apt/lists/*

# Install diesel CLI
RUN curl --proto '=https' --tlsv1.2 -LsSf https://github.com/diesel-rs/diesel/releases/latest/download/diesel_cli-installer.sh | sh

# Add cargo bin to PATH
ENV PATH="/root/.cargo/bin:$PATH"

# Set working directory
WORKDIR /hyperswitch

# Copy migration files and diesel config from the workspace
COPY ./migrations/ ./migrations/
COPY ./diesel.toml ./diesel.toml
COPY ./crates/diesel_models/src/schema.rs ./crates/diesel_models/src/schema.rs
COPY ./crates/diesel_models/drop_id.patch ./crates/diesel_models/drop_id.patch

# Create the 'app' user and group
RUN useradd --user-group --system --no-create-home --no-log-init app

# Change ownership of working directory to app user
RUN chown -R app:app /hyperswitch

USER app:app

# Default command to run migrations
# Note: DATABASE_URL environment variable should be provided at runtime
CMD ["diesel", "migration", "run"]
