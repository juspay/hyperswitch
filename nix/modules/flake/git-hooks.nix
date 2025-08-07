{ inputs, ... }:
{
  imports = [
    (inputs.git-hooks + /flake-module.nix)
  ];
  perSystem = { ... }: {
    pre-commit.settings.hooks = {
      nixpkgs-fmt.enable = true;
    };
  };
}
