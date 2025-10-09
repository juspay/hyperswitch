# Migration Runner Dockerfile for Hyperswitch
# This image contains migration files and diesel CLI for offline migration execution

FROM debian:trixie-slim

# Install necessary packages
RUN apt-get update && apt-get install -y \
    curl \
    tar \
    xz-utils \
    && rm -rf /var/lib/apt/lists/*

# Create the 'app' user and group
RUN useradd --user-group --system --create-home --no-log-init app

# Switch to the 'app' user
USER app:app

# Install diesel CLI
RUN curl --proto '=https' --tlsv1.2 -LsSf https://github.com/diesel-rs/diesel/releases/latest/download/diesel_cli-installer.sh | sh

ENV PATH="/home/app/.cargo/bin:$PATH"

# Set working directory
WORKDIR /hyperswitch

# Copy migration files and diesel config from the workspace
COPY --chown=app:app ./migrations/ ./migrations/
COPY --chown=app:app ./diesel.toml ./diesel.toml
COPY --chown=app:app ./crates/diesel_models/src/schema.rs ./crates/diesel_models/src/schema.rs
COPY --chown=app:app ./crates/diesel_models/drop_id.patch ./crates/diesel_models/drop_id.patch

# Copy the migration runner script
COPY --chown=app:app ./scripts/migration_runner_entrypoint.sh ./migration_runner_entrypoint.sh

# Default command to run migrations
CMD ["./migration_runner_entrypoint.sh"]
