{ inputs, lib, ... }:
{
  imports = [
    inputs.process-compose-flake.flakeModule
  ];
  perSystem = { pkgs, system, ... }: {
    /* For running external services
        - Redis
        - Postgres
        - Superposition
    */
    process-compose."ext-services" = { config, ... }:
      let
        developmentToml = lib.importTOML (inputs.self + /config/development.toml);
        databaseName = developmentToml.master_database.dbname;
        databaseUser = developmentToml.master_database.username;
        databasePass = developmentToml.master_database.password;
        superpositionEndpoint = developmentToml.superposition.endpoint;
        superpositionOrgId = developmentToml.superposition.org_id;
        superpositionWorkspaceId = developmentToml.superposition.workspace_id;
        superpositionWorkspaceSchemaName = "${superpositionOrgId}_${superpositionWorkspaceId}";
        superpositionPort =
          let
            matches = builtins.match "http://localhost:([0-9]+).*" superpositionEndpoint;
          in
          if matches == null then 8082 else builtins.fromJSON (builtins.head matches);
        superpositionDatabaseName = "config";
        superpositionPackage = inputs.superposition.packages.${system}.superposition;
        superpositionSql = inputs.superposition + /superposition.sql;
        superpositionWorkspaceSql = pkgs.runCommand "superposition-workspace-${superpositionWorkspaceSchemaName}.sql" { } ''
          ${pkgs.gnused}/bin/sed 's/{replaceme}/${superpositionWorkspaceSchemaName}/g' \
            ${inputs.superposition}/workspace_template.sql > "$out"
        '';
      in
      {
        imports = [ inputs.services-flake.processComposeModules.default ];
        services.redis."r1".enable = true;

        /* Postgres
            - Create an user and grant all privileges
            - Create a database
        */
        services.postgres."p1" = {
          enable = true;
          initialScript = {
            before = "CREATE USER ${databaseUser} WITH PASSWORD '${databasePass}' SUPERUSER CREATEDB CREATEROLE INHERIT LOGIN;";
            after = "GRANT ALL PRIVILEGES ON DATABASE ${databaseName} to ${databaseUser};";
          };
          initialDatabases = [
            { name = databaseName; }
          ];
        };

        settings.processes.superposition-db-init = {
          namespace = "superposition";
          command = pkgs.writeShellApplication {
            name = "setup-superposition-db";
            runtimeInputs = [
              config.services.postgres.p1.package
            ];
            text = ''
              db_exists="$(psql -h 127.0.0.1 -p 5432 -U ${databaseUser} -d postgres -Atc "SELECT 1 FROM pg_database WHERE datname = '${superpositionDatabaseName}'")"

              if [ "$db_exists" != "1" ]; then
                createdb -h 127.0.0.1 -p 5432 -U ${databaseUser} ${superpositionDatabaseName}
              fi

              psql \
                -h 127.0.0.1 \
                -p 5432 \
                -U ${databaseUser} \
                -d ${superpositionDatabaseName} \
                -v ON_ERROR_STOP=1 \
                -f "${superpositionSql}"

              psql \
                -h 127.0.0.1 \
                -p 5432 \
                -U ${databaseUser} \
                -d ${superpositionDatabaseName} \
                -v ON_ERROR_STOP=1 \
                <<'SQL'
              INSERT INTO superposition.organisations (
                id,
                name,
                created_by,
                admin_email,
                updated_by
              ) VALUES (
                '${superpositionOrgId}',
                '${superpositionOrgId}',
                'admin@localorg.io',
                'admin@localorg.io',
                'admin@localorg.io'
              )
              ON CONFLICT (id) DO NOTHING;
              SQL

              psql \
                -h 127.0.0.1 \
                -p 5432 \
                -U ${databaseUser} \
                -d ${superpositionDatabaseName} \
                -v ON_ERROR_STOP=1 \
                -f "${superpositionWorkspaceSql}"

              psql \
                -h 127.0.0.1 \
                -p 5432 \
                -U ${databaseUser} \
                -d ${superpositionDatabaseName} \
                -v ON_ERROR_STOP=1 \
                <<'SQL'
              INSERT INTO superposition.workspaces (
                organisation_id,
                organisation_name,
                workspace_name,
                workspace_schema_name,
                workspace_status,
                workspace_admin_email,
                created_by,
                last_modified_by,
                mandatory_dimensions
              ) VALUES (
                '${superpositionOrgId}',
                '${superpositionOrgId}',
                '${superpositionWorkspaceId}',
                '${superpositionWorkspaceSchemaName}',
                'ENABLED',
                'admin@localorg.io',
                'admin@localorg.io',
                'admin@localorg.io',
                null
              )
              ON CONFLICT (organisation_id, workspace_name) DO UPDATE SET
                workspace_schema_name = EXCLUDED.workspace_schema_name,
                workspace_status = EXCLUDED.workspace_status,
                last_modified_by = EXCLUDED.last_modified_by,
                last_modified_at = CURRENT_TIMESTAMP;
              SQL
            '';
          };
          depends_on.p1.condition = "process_healthy";
        };

        settings.processes.superposition = {
          namespace = "superposition";
          command = pkgs.writeShellApplication {
            name = "start-superposition";
            runtimeInputs = [ superpositionPackage ];
            text = ''
              export PORT="${toString superpositionPort}"
              export APP_ENV="DEV"
              export DB_USER="${databaseUser}"
              export DB_PASSWORD="${databasePass}"
              export DB_HOST="127.0.0.1:5432"
              export DB_NAME="${superpositionDatabaseName}"
              export REDIS_URL="redis://127.0.0.1:6379"
              export REDIS_POOL_SIZE="10"
              export REDIS_MAX_ATTEMPTS="10"
              export REDIS_CONN_TIMEOUT="1000"
              export REDIS_KEY_TTL="604800"
              export CAC_HOST="${superpositionEndpoint}"
              export API_HOSTNAME="${superpositionEndpoint}"
              export SUPERPOSITION_VERSION="v0.102.0"
              export SUPERPOSITION_TOKEN="${developmentToml.superposition.token}"
              export HOSTNAME="hyperswitch-local-superposition"
              export ACTIX_KEEP_ALIVE="120"
              export MAX_DB_CONNECTION_POOL_SIZE="3"
              export TENANT_MIDDLEWARE_EXCLUSION_LIST="/health,/assets/favicon.ico,/pkg/frontend.js,/pkg,/pkg/frontend_bg.wasm,/pkg/tailwind.css,/pkg/style.css,/assets,/admin,/oidc/login,/admin/organisations,/organisations,/organisations/switch/{organisation_id},/"
              export SERVICE_PREFIX=""
              export AUTH_PROVIDER="DISABLED"
              export AUTH_Z_PROVIDER="DISABLED"
              export WORKER_ID="1"
              export ALLOW_SAME_KEYS_OVERLAPPING_CTX="true"
              export ALLOW_DIFF_KEYS_OVERLAPPING_CTX="true"
              export ALLOW_SAME_KEYS_NON_OVERLAPPING_CTX="true"
              export SUPERPOSITION_METRICS_ENABLED="false"

              cd "${inputs.superposition}"
              exec superposition
            '';
          };
          depends_on = {
            p1.condition = "process_healthy";
            r1.condition = "process_healthy";
            superposition-db-init.condition = "process_completed_successfully";
          };
          availability = {
            restart = "on_failure";
            max_restarts = 5;
          };
          readiness_probe = {
            http_get = {
              host = "127.0.0.1";
              port = superpositionPort;
              path = "/health";
            };
            initial_delay_seconds = 2;
            period_seconds = 5;
            timeout_seconds = 4;
            failure_threshold = 12;
          };
        };

        settings.processes.superposition-init = {
          namespace = "superposition";
          command = pkgs.writeShellApplication {
            name = "seed-superposition";
            runtimeInputs = with pkgs; [
              bash
              curl
              jq
              yq-go
            ];
            text = ''
              export SUPERPOSITION_URL="${superpositionEndpoint}"
              export SEED_FILE="${inputs.self}/config/superposition_seed.toml"
              export WORKSPACE_ID="${superpositionWorkspaceId}"
              export ORG_ID="${superpositionOrgId}"

              ${pkgs.bash}/bin/bash "${inputs.self}/scripts/seed_superposition.sh"
            '';
          };
          depends_on.superposition.condition = "process_healthy";
        };
      };
  };
}
