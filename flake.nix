{
  description = "hyperswitch";

  inputs = {
    cargo2nix.url = "github:cargo2nix/cargo2nix/release-0.11.0";
    nixpkgs.url      = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url  = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
    # nixpkgs.follows = "cargo2nix/nixpkgs";
    # flake-utils.follows = "cargo2nix/flake-utils";
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

  outputs = inputs: with inputs;
    flake-utils.lib.eachDefaultSystem (system:
        let
        pkgs = import nixpkgs {
            inherit system;
            overlays = [cargo2nix.overlays.default (import rust-overlay)];
        };

        rustPkgs = pkgs.rustBuilder.makePackageSet {
            rustVersion = "1.65.0";
            packageFun = import ./Cargo.nix;
        };

        in rec {
            packages = {
                router = (rustPkgs.workspace.router {}).bin;
                # router-scheduler = pkgs.lib.elemAt (rustPkgs.workspace.router {}).all 1;
                default = packages.router;
            };
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
