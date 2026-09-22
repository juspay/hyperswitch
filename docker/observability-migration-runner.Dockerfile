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
RUN curl --proto '=https' --tlsv1.2 -LsSf https://github.com/diesel-rs/diesel/releases/download/v2.3.5/diesel_cli-installer.sh | sh

ENV PATH="/home/app/.cargo/bin:$PATH"

# Set working directory
WORKDIR /hyperswitch

COPY --chown=app:app ./crates/observability/migrations/ ./crates/observability/migrations/
COPY --chown=app:app ./crates/observability/diesel.toml ./crates/observability/diesel.toml
COPY --chown=app:app ./crates/diesel_models/src/observability/schema.rs ./crates/diesel_models/src/observability/schema.rs

COPY --chown=app:app ./scripts/migration_runner_entrypoint.sh ./migration_runner_entrypoint.sh

ENV DIESEL_CONFIG_FILE="/hyperswitch/crates/observability/diesel.toml"

CMD ["./migration_runner_entrypoint.sh"]
