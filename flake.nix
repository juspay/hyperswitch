{
  description = "hyperswitch";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";

    # TODO: Move away from these to https://github.com/juspay/rust-flake
    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = nixpkgs.lib.systems.flakeExposed;
      perSystem = { self', pkgs, system, ... }:
        let
          rustVersion = "1.65.0";
          frameworks = pkgs.darwin.apple_sdk.frameworks;
        in
        {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            overlays = [ inputs.cargo2nix.overlays.default (import inputs.rust-overlay) ];
          };
          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              openssl
              pkg-config
              exa
              fd
              rust-bin.stable.${rustVersion}.default
            ] ++ lib.optionals stdenv.isDarwin [ frameworks.CoreServices frameworks.Foundation ]; # arch might have issue finding these libs.

          };
        };
    };
}
