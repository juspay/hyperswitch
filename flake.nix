{
  description = "orca devshell";

  inputs = {
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url  = "github:numtide/flake-utils";
  };

  # cache for faster access
  nixConfig.extra-substituters = [
    "https://iog.cachix.org"
    "https://hydra.iohk.io"
  ];
  nixConfig.extra-trusted-public-keys = [
    "iog.cachix.org-1:nYO0M9xTk/s5t1Bs9asZ/Sww/1Kt/hRhkLP0Hhv/ctY="
    "hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ="
  ];

  outputs = { self, nixpkgs, rust-overlay, flake-utils, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        frameworks = pkgs.darwin.apple_sdk.frameworks;

      in
      with pkgs;
      {
        devShells.default = mkShell {

          buildInputs =  [
            openssl
            pkg-config
            exa
            fd
            rust-bin.stable."1.65.0".default
          ] ++ lib.optionals stdenv.isDarwin [ frameworks.CoreServices frameworks.Foundation ]; # arch might have issue finding these libs.

        };
      }
    );
}
