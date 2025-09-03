{
  debug = true;
  perSystem = { config, pkgs, self', ... }:
    let
      rustMsrv = config.rust-project.cargoToml.workspace.package.rust-version;
      rustDevVersion = config.rust-project.toolchain.version;
    in
    {
      devShells = {
        default =
          pkgs.mkShell {
            name = "hyperswitch-shell";
            meta.description = "Environment for Hyperswitch development";
            inputsFrom = [ config.pre-commit.devShell ];
            packages = with pkgs; [
              diesel-cli
              just
              jq
              openssl
              pkg-config
              postgresql # for libpq
              protobuf
            ];
          };
        dev = pkgs.mkShell {
          name = "hyperswitch-dev-shell";
          meta.description = "Environment for Hyperswitch development with additional tools";
          inputsFrom = [ self'.devShells.default ];
          packages = with pkgs; [
            cargo-watch
            nixd
            rust-bin.stable.${rustDevVersion}.default
            swagger-cli
          ];
          shellHook = ''
            echo 1>&2 "Ready to work on hyperswitch!"
            rustc --version
            export OPENSSL_DIR="${pkgs.openssl.dev}"
            export OPENSSL_LIB_DIR="${pkgs.openssl.out}/lib"
          '';
        };
        qa = pkgs.mkShell {
          name = "hyperswitch-qa-shell";
          meta.description = "Environment for Hyperswitch QA";
          inputsFrom = [ self'.devShells.dev ];
          packages = with pkgs; [
            cypress
            nodejs
            parallel
          ];
        };
      };
    };
}
