{
  description = "hyperswitch";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";

    # TODO: Move away from these to https://github.com/juspay/rust-flake
    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake { inherit inputs; } {
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
              openssl
              pkg-config
              exa
              fd
              rust-bin.stable.${rustVersion}.default
            ] ++ lib.optionals stdenv.isDarwin [
              # arch might have issue finding these libs.
              frameworks.CoreServices
              frameworks.Foundation
            ];
          };
        };
    };
}
