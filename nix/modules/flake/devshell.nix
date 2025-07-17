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
            swagger-cli

            # The order in which these rust package appear is important
            rustStablePkg
            rustFmtNightlyPkg
          ];
          shellHook = ''
            echo 1>&2 "Ready to work on hyperswitch!"
            rustc --version
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

        # FIXME: Once `euclid_wasm` is buildable in devShell env we can move this to `devShells.dev`
        test-euclid_wasm =
          let
            clangUnwrapped = pkgs.llvmPackages_latest.clang-unwrapped;
            bintoolsUnwrapped = pkgs.llvmPackages_latest.bintools-unwrapped;
            wasm32 = "wasm32-unknown-unknown";
          in
          pkgs.mkShell {
            name = "hyperswitch-shell-euclid_wasm";
            meta.description = "Test shell env for euclid_wasm";
            inputsFrom = [ self'.devShells.dev ];
            packages = with pkgs; [
              wasm-pack
              wasm-bindgen-cli
              clangUnwrapped
              bintoolsUnwrapped
              lld
              libpq
            ] ++ lib.optionals pkgs.stdenv.isDarwin (with pkgs;
              [
                libiconv
                apple-sdk_11
              ]);

            shellHook = ''
              rustfmt --version
              cargo --version
              echo 1>&2 ""
              echo 1>&2 "Note: This shell is for testing \`euclid_wasm\` in a local build environment."
              echo 1>&2 "Basically the env is a rough pick from \`crates/euclid_wasm/crate.nix\` though that is also broken at the moment."
            '';

            CC_wasm32_unknown_unknown = lib.getExe' clangUnwrapped "clang";
            CXX_wasm32_unknown_unknown = lib.getExe' clangUnwrapped "clang++";
            AR_wasm32_unknown_unknown = lib.getExe' bintoolsUnwrapped "llvm-ar";
            AS_wasm32_unknown_unknown = lib.getExe' bintoolsUnwrapped "llvm-as";
            STRIP_wasm32_unknown_unknown = lib.getExe' bintoolsUnwrapped "llvm-strip";

            CARGO_BUILD_TARGET = wasm32;
            CPATH = "${lib.getLib clangUnwrapped}/lib/clang/${lib.versions.major (lib.getVersion clangUnwrapped)}/include";
            RUSTFLAGS = "-C linker=rust-lld";

          };

      };
    };
}
