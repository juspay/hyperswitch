{ inputs, ... }:
{
  imports = [
    inputs.rust-flake.flakeModules.default
    inputs.rust-flake.flakeModules.nixpkgs
  ];
  perSystem = { pkgs, ... }: {
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
    };
  };
}
