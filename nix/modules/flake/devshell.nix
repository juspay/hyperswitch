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
            clangLib = lib.getLib clangPKG;
            majorVer = lib.versions.major (lib.getVersion clangPKG);
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
              pkg-config
              wasm-pack
              libpq
            ];

            # To avoid issues if someone is having `homebrew's` openssl installed
            OPENSSL_DIR = "${pkgs.openssl.dev}";
            OPENSSL_LIB_DIR = "${pkgs.openssl.out}/lib";

            CPATH = "${clangLib}/lib/clang/${majorVer}/include";
            TARGET_CC = lib.getExe' clangPKG "clang";

            shellHook = ''
              echo 1>&2 "Ready to work on hyperswitch!"
              rustc --version
              rustfmt --version

              # FIXME: This is a workaround for the error `rust-lld: error: unable to find library -lpq`
              # Have tried setting variables like `LD_LIBRARY_PATH`, `LIBRARY_PATH`, `PKG_CONFIG_PATH` but nothing worked
              # Also like LD_FLAGS, LDFLAGS, CFLAGS, CXXFLAGS, etc. didn't work
              # `PATH` has to be exported in the shellHook as Nix doesn't modify user's Environment variables
              # homebrew's libpq bin path includes `pg_config` and `ecpg` as extra binaries from nix's postgresql
              # pg_config in nix is at `pkgs.postgresql.pg_config` and `ecpg` is at `pkgs.postgresql.dev`
              # lib dir maybe req to be compared to resolve the issue ......

              # export PATH="${pkgs.postgresql.pg_config}/bin:${pkgs.postgresql.dev}/bin:${pkgs.postgresql}/bin:$PATH"
              HBREW_LIBPQ_PATH="${lib.optionalString (pkgs.stdenv.isDarwin) "/opt/homebrew/opt/libpq/bin"}"
              export PATH="$HBREW_LIBPQ_PATH:$PATH"
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
