{
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-20.09";
  inputs.naersk = {
    url = "github:nmattia/naersk";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  inputs.rust-overlay = {
    url = "github:oxalica/rust-overlay";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  inputs.gitignore = {
    url = "github:hercules-ci/gitignore.nix";
    flake = false;
  };

  outputs = { nixpkgs, rust-overlay, naersk, gitignore, ... }: rec {
    packages.x86_64-linux = let
      pkgs = import nixpkgs {
        system = "x86_64-linux";
        overlays = [ rust-overlay.overlay ];
      };
      gitignore-lib = import gitignore { inherit (pkgs) lib; };
      inherit (gitignore-lib) gitignoreSource;
      rust = pkgs.rust-bin.nightly.latest.default;
      naersk-lib = naersk.lib.x86_64-linux.override {
        cargo = rust;
        rustc = rust;
      };
    in rec {
      discord-game-sdk = pkgs.callPackage ./game-sdk.nix {};
      mpdiscord = naersk-lib.buildPackage {
        root = gitignoreSource ./.;
        nativeBuildInputs = with pkgs; [ llvmPackages.llvm pkgconfig ];
        buildInputs = with pkgs; [ discord-game-sdk stdenv.cc.libc openssl ];
        override = x: (x // {
          DISCORD_GAME_SDK_PATH = discord-game-sdk;
          LIBCLANG_PATH = "${pkgs.llvmPackages.libclang}/lib";
          preConfigure = ''
          export BINDGEN_EXTRA_CLANG_ARGS="-isystem ${pkgs.clang}/resource-root/include $NIX_CFLAGS_COMPILE"
          '';
        });
      };
    };

    defaultPackage.x86_64-linux = packages.x86_64-linux.mpdiscord;
    devShell.x86_64-linux = builtins.head packages.x86_64-linux.mpdiscord.builtDependencies;
  };
}
