{
  description = "hyperswitch";

  inputs = {
    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-parts.url = "github:hercules-ci/flake-parts";
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = inputs@{ self, nixpkgs, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = nixpkgs.lib.systems.flakeExposed;
      perSystem = { self', pkgs, system, ... }:
        let
          rustVersion = "1.65.0";
          rustPkgs = pkgs.rustBuilder.makePackageSet {
            inherit rustVersion;
            packageFun = import ./Cargo.nix;
          };
          frameworks = pkgs.darwin.apple_sdk.frameworks;
        in
        {
          _module.args.pkgs = import nixpkgs {
            inherit system;
            overlays = [ inputs.cargo2nix.overlays.default (import inputs.rust-overlay) ];
          };
          packages = rec {
            router = (rustPkgs.workspace.router { }).bin;
            default = router;
          };
          apps = {
            router-scheduler = {
              type = "app";
              program = "${self'.packages.router}/bin/scheduler";
            };
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
