{ inputs, ... }:
{
  imports = [
    inputs.rust-flake.flakeModules.default
    inputs.rust-flake.flakeModules.nixpkgs
  ];
  perSystem = { pkgs, lib, config, ... }: {
    rust-project = {
      toolchain = pkgs.rust-bin.fromRustupToolchain {
        channel = "1.88.0";
        components = [
          # Picked from: <https://rust-lang.github.io/rustup/concepts/components.html>
          # TODO: only keep required after euclid_wasm is buildable in devShell env
          "cargo"
          "clippy"
          "llvm-bitcode-linker"
          "llvm-tools"
          "rust-analyzer"
          "rust-docs"
          "rust-src"
          "rust-std"
          "rustc-dev"
        ];
        profile = "minimal"; # Has to be minimal to avoid overriding nightly `rustfmt`
        targets = [ "wasm32-unknown-unknown" ];
      };
      crateNixFile = "crate.nix";
      src = lib.cleanSourceWith {
        src =
          let
            # Like crane's filterCargoSources, but doesn't blindly include all TOML files!
            filterCargoSources = path: type:
              config.rust-project.crane-lib.filterCargoSources path type
              && !(lib.hasSuffix ".toml" path && !lib.hasSuffix "Cargo.toml" path);
          in
          # TODO: Put a more thorough filter here
          lib.cleanSourceWith {
            src = inputs.self;
            filter = path: type:
              filterCargoSources path type
              || lib.hasSuffix "crate.nix" path
              || "${inputs.self}/flake.nix" == path
              || "${inputs.self}/flake.lock" == path
              || "${inputs.self}/README.md" == path
              || lib.hasPrefix "${inputs.self}/nix/" path
              || lib.hasPrefix "${inputs.self}/crates/" path
            ;
          };
      };
    };
  };
}
