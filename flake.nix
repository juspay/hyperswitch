{
  description = "hyperswitch";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";

    rust-flake.url = "github:juspay/rust-flake";

    nixos-unified.url = "github:srid/nixos-unified";

    git-hooks.url = "github:cachix/git-hooks.nix";
    git-hooks.flake = false;

    process-compose-flake.url = "github:Platonic-Systems/process-compose-flake";
    services-flake.url = "github:juspay/services-flake";
  };

  outputs = inputs:
    inputs.nixos-unified.lib.mkFlake
      { inherit inputs; root = ./.; };
}
