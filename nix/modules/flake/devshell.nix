{
  debug = true;
  perSystem = { config, pkgs, lib, self', ... }:
    let
      rustMsrv = config.rust-project.cargoToml.workspace.package.rust-version;
      rustStablePkg = config.rust-project.toolchain;
      rustFmtNightlyPkg = pkgs.rust-bin.nightly.latest.minimal.override {
        extensions = [ "clippy" "rustfmt" ];
      };
    in
    {
      devShells = {
        default = pkgs.mkShell {
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
        dev =
          let
            clangPKG = pkgs.llvmPackages_latest.clang-unwrapped;
          in
          pkgs.mkShell {
            name = "hyperswitch-dev-shell";
            meta.description = "Environment for Hyperswitch development with additional tools";
            inputsFrom = [ self'.devShells.default ];

            packages = with pkgs; [
              cargo-watch
              nixd
              swagger-cli

              # The order in which these rust package appear is important
              rustStablePkg
              rustFmtNightlyPkg

              # euclid_wasm dependencies
              clangPKG
              pkgconf
              wasm-pack
            ];

            # To avoid issues if someone is having `homebrew's` openssl installed
            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

            PATH = "${clangPKG}/bin:$PATH";

            TARGET_CC = lib.getExe' clangPKG "clang";
            CPATH = "${lib.getLib clangPKG}/lib/clang/${lib.versions.major (lib.getVersion clangPKG)}/include";

            shellHook = ''
              echo 1>&2 "Ready to work on hyperswitch!"
              rustc --version
              rustfmt --version
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
