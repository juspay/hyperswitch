{ inputs, ... }:
{
  imports = [
    inputs.rust-flake.flakeModules.default
    inputs.rust-flake.flakeModules.nixpkgs
  ];
  perSystem = { pkgs, ... }: {
    rust-project.toolchain = pkgs.rust-bin.fromRustupToolchain {
      channel = "1.88.0";
    };
  };
}
