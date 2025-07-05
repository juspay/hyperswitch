{ inputs, lib, ... }:
{
  imports = [
    inputs.process-compose-flake.flakeModule
  ];
  perSystem = { ... }: {
    /* For running external services
        - Redis
        - Postgres
    */
    process-compose."ext-services" =
      let
        developmentToml = lib.importTOML (inputs.self + /config/development.toml);
        databaseName = developmentToml.master_database.dbname;
        databaseUser = developmentToml.master_database.username;
        databasePass = developmentToml.master_database.password;
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
      };
  };
}
