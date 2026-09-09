# Migration runner for the observability database.
#
# A sibling of migration-runner.Dockerfile rather than a flag on it: the two lineages target
# different databases, and this image carries no root `migrations/` and no `diesel.toml`, so
# pointing it at hyperswitch_db does nothing rather than something surprising.
#
# Run it as a Job with `parallelism: 1`. Diesel does not serialise concurrent migrations -- two at
# once leave one succeeding and the other exiting 1 on a Postgres catalogue conflict, which reads
# as a broken deployment rather than a race.

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

COPY --chown=app:app ./crates/observability/migrations/ ./crates/observability/migrations/
COPY --chown=app:app ./crates/observability/diesel.toml ./crates/observability/diesel.toml
COPY --chown=app:app ./crates/diesel_models/src/observability/schema.rs ./crates/diesel_models/src/observability/schema.rs

COPY --chown=app:app ./scripts/migration_runner_entrypoint.sh ./migration_runner_entrypoint.sh

# The entrypoint defaults to the root lineage, so this image names its own.
ENV MIGRATION_DIR="/hyperswitch/crates/observability/migrations"
ENV DIESEL_CONFIG_FILE="/hyperswitch/crates/observability/diesel.toml"

CMD ["./migration_runner_entrypoint.sh"]
