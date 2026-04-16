# Pre-commit hooks via https://github.com/cachix/git-hooks.nix
#
# These hooks run automatically on `git commit` (installed via the devShell)
# and in CI via the `pre-commit-run` derivation (built by `nix flake check`).
#
# NOTE: Clippy is intentionally NOT included here. `cargo clippy` is a compiler
# tool that must compile the entire workspace to perform its analysis. This makes
# it incompatible with the `pre-commit-run` Nix sandbox derivation for several
# reasons:
#
#   1. Compilation requires ALL native build dependencies (openssl, rdkafka,
#      protobuf, etc.) to be available in the sandbox, making the derivation
#      fragile and tightly coupled to the project's build environment.
#
#   2. Some vendored git dependencies (e.g. connector-service crates) use
#      `[lints] workspace = true` in their Cargo.toml, which fails to resolve
#      outside their original workspace context in the vendor dir.
#
#   3. Build scripts in vendored crates (e.g. superposition_core, rdkafka-sys)
#      run their own cargo metadata or try to build native libs from source,
#      causing cascading failures in the sandbox.
#
#   4. Full compilation from scratch (no target/ cache) takes 30+ minutes,
#      making it impractical as a flake check.
#
# Clippy is run in CI via GitHub Actions (CI-pr.yml, CI-push.yml) where the
# host environment provides all build deps and a warm Rust cache.
{ inputs, ... }:
{
  imports = [
    (inputs.git-hooks + /flake-module.nix)
  ];
  perSystem = { pkgs, ... }: {
    pre-commit.settings = {
      hooks = {
        nixpkgs-fmt.enable = true;

        # Use nightly rustfmt (the project relies on nightly-only formatting
        # features). packageOverrides swaps the toolchain for this hook only,
        # without affecting the project's stable Rust toolchain.
        rustfmt = {
          enable = true;
          packageOverrides = {
            cargo = pkgs.rust-bin.nightly.latest.minimal;
            rustfmt = pkgs.rust-bin.nightly.latest.rustfmt;
          };
        };
      };
    };
  };
}
