{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.flake-utils.url = "github:numtide/flake-utils";
  inputs.naersk = {
    url = "github:nmattia/naersk";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = { nixpkgs, flake-utils, rust-overlay, naersk, self, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default naersk.overlays.default ];
        };
        rust = pkgs.rust-bin.nightly.latest.default;
        inherit (pkgs.rust-bin.nightly.latest) cargo;
        naersk-lib = pkgs.naersk.override {
          inherit cargo;
          rustc = rust;
        };
        rust-dev = rust.override {
          extensions = [ "rust-src" "clippy" ];
        };
      in rec {
        packages = {
          mpdiscord = naersk-lib.buildPackage rec {
            name = "mpdiscord";
            version = "unstable";
            root = ./.;
            nativeBuildInputs = with pkgs; [ llvmPackages.llvm pkg-config ];
            buildInputs = with pkgs; [ stdenv.cc.libc openssl ];
            overrideMain = x: (x // rec {
              name = "${pname}-${version}";
              pname = "mpdiscord";
              version =
                let
                  rev = self.shortRev or null;
                in
                  if rev != null then "unstable-${rev}" else "dirty";
            });
          };
        };

        defaultPackage = packages.mpdiscord;

        devShell = pkgs.mkShell {
          inputsFrom = packages.mpdiscord.builtDependencies;
          nativeBuildInputs = with pkgs; [ rust-dev ];
        };
      }
    );
}
