{
  inputs.naersk.url = "github:nmattia/naersk";
  inputs.rust-overlay.url = "github:oxalica/rust-overlay";
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
      rust = pkgs.rust-bin.nightly.latest.rust;
      naersk-lib = naersk.lib.x86_64-linux.override {
        cargo = rust;
        rustc = rust;
      };
    in rec {
      discord-game-sdk = pkgs.callPackage ./game-sdk.nix {};
      mpdiscord = naersk-lib.buildPackage {
        root = gitignoreSource ./.;
        buildInputs = [ discord-game-sdk ];
      };
    };

    defaultPackage.x86_64-linux = packages.x86_64-linux.mpdiscord;
  };
}
