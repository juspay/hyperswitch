{ pkgs, lib, ... }:
let
  wasm32 = "wasm32-unknown-unknown";
  clangUnwrapped = pkgs.llvmPackages_latest.clang-unwrapped;
  bintoolsUnwrapped = pkgs.llvmPackages_latest.bintools-unwrapped;
in
{
  autoWire = [ "crate" ];
  crane = {
    args =
      {
        CC_wasm32_unknown_unknown = lib.getExe' clangUnwrapped "clang";
        CXX_wasm32_unknown_unknown = lib.getExe' clangUnwrapped "clang++";
        AR_wasm32_unknown_unknown = lib.getExe' bintoolsUnwrapped "llvm-ar";
        AS_wasm32_unknown_unknown = lib.getExe' bintoolsUnwrapped "llvm-as";
        STRIP_wasm32_unknown_unknown = lib.getExe' bintoolsUnwrapped "llvm-strip";

        CARGO_BUILD_TARGET = wasm32;
        CPATH = "${lib.getLib clangUnwrapped}/lib/clang/${lib.versions.major (lib.getVersion clangUnwrapped)}/include";
        RUSTFLAGS = "-C linker=rust-lld";

        buildInputs =
          with pkgs;[
            bintoolsUnwrapped
            clangUnwrapped

            lld
            libpq
            openssl
            pkg-config
            postgresql # for libpq
            wasm-pack
            wasm-bindgen-cli
          ]
          ++ lib.optionals pkgs.stdenv.isDarwin (with pkgs;[
            # Additional darwin specific inputs can be set here
            libiconv
            apple-sdk_11
          ]);
      };
  };
}
