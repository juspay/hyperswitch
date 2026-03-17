{ inputs, ... }:
{
  imports = [
    (inputs.git-hooks + /flake-module.nix)
  ];
  perSystem = { pkgs, ... }: {
    pre-commit.settings.hooks = {
      nixpkgs-fmt.enable = true;
      rustfmt = {
        enable = true;
        # Nightly required: .rustfmt.toml uses unstable options (group_imports, imports_granularity)
        packageOverrides = {
          cargo = pkgs.rust-bin.nightly.latest.minimal;
          rustfmt = pkgs.rust-bin.nightly.latest.rustfmt;
        };
      };
    };
  };
}
