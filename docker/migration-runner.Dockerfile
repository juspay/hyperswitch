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

# Default command to run migrations
# Supports both DATABASE_URL or individual POSTGRES_HOST, POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB
CMD ["sh", "-c", "if [ -z \"$DATABASE_URL\" ]; then if [ -z \"$POSTGRES_HOST\" ] || [ -z \"$POSTGRES_USER\" ] || [ -z \"$POSTGRES_PASSWORD\" ] || [ -z \"$POSTGRES_DB\" ]; then echo 'Error: Either DATABASE_URL or all of POSTGRES_HOST, POSTGRES_USER, POSTGRES_PASSWORD, POSTGRES_DB must be provided'; exit 1; fi; export DATABASE_URL=\"postgresql://$POSTGRES_USER:$POSTGRES_PASSWORD@$POSTGRES_HOST:5432/$POSTGRES_DB\"; fi; diesel migration run"]
