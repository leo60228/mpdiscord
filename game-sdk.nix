{ stdenv, fetchzip }:
stdenv.mkDerivation rec {
  pname = "discord-game-sdk";
  version = "2.5.6";

  src = fetchzip {
    url = "https://dl-game-sdk.discordapp.net/${version}/discord_game_sdk.zip";
    hash = "sha256-iyZTGspnVl0O3nHGELk2tgVYYHRPQCBFI5EMXMFXApY=";
    stripRoot = false;
  };

  dontBuild = true;

  installPhase = ''
  mkdir -p $out/lib
  cp $src/lib/x86_64/discord_game_sdk.so $out/lib/libdiscord_game_sdk.so
  '';
}
