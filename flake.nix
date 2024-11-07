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
          rustVersion = cargoToml.workspace.package.rust-version;
          frameworks = pkgs.darwin.apple_sdk.frameworks;
        in
        {
          _module.args.pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ inputs.cargo2nix.overlays.default (import inputs.rust-overlay) ];
          };
          devShells.default = pkgs.mkShell {
            name = "hyperswitch-shell";
            packages = with pkgs; [
              just
              nixd
              openssl
              pkg-config
              rust-bin.stable.${rustVersion}.default
            ] ++ lib.optionals stdenv.isDarwin [
              # arch might have issue finding these libs.
              frameworks.CoreServices
              frameworks.Foundation
            ];
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
