{
  description = "hyperswitch";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";

    # TODO: Move away from these to https://github.com/juspay/rust-flake
    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    rust-overlay.url = "github:oxalica/rust-overlay";

    process-compose-flake.url = "github:Platonic-Systems/process-compose-flake";
    services-flake.url = "github:juspay/services-flake";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
      imports = [ inputs.process-compose-flake.flakeModule ];
      systems = inputs.nixpkgs.lib.systems.flakeExposed;
      perSystem = { self', pkgs, lib, system, ... }:
        let
          cargoToml = lib.importTOML ./Cargo.toml;
          rustDevVersion = "1.87.0";
          rustMsrv = cargoToml.workspace.package.rust-version;

          # Common packages
          base = with pkgs; [
            diesel-cli
            just
            jq
            openssl
            pkg-config
            postgresql # for libpq
            protobuf
          ];

          # Minimal packages for running hyperswitch
          runPackages = base ++ (with pkgs; [
            rust-bin.stable.${rustMsrv}.default
          ]);

          # Development packages
          devPackages = base ++ (with pkgs; [
            cargo-watch
            nixd
            rust-bin.stable.${rustDevVersion}.default
            swagger-cli
          ]);

          # QA packages
          qaPackages = devPackages ++ (with pkgs; [
            cypress
            nodejs
            parallel
          ]);

        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.cargo2nix.overlays.default (import inputs.rust-overlay) ];
          };

          # Minimal shell
          devShells.default = pkgs.mkShell {
            name = "hyperswitch-shell";
            packages = base;
          };

          # Development shell
          devShells.dev = pkgs.mkShell {
            name = "hyperswitch-dev-shell";
            packages = devPackages;
          };

          # QA development shell
          devShells.qa = pkgs.mkShell {
            name = "hyperswitch-qa-shell";
            packages = qaPackages;
          };

          /* For running external services
              - Redis
              - Postgres
          */
          process-compose."ext-services" =
            let 
              developmentToml = lib.importTOML ./config/development.toml;
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
    };
}
