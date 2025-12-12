self: {
  lib,
  pkgs,
  config,
  ...
}:
with lib; let
  cfg = config.programs.zarumet;
  zarumet = self.packages.${pkgs.stdenv.hostPlatform.system}.default;

  hexColorOption = default:
    mkOption {
      type = types.string;
      default = default;
      description = "Hex color value";
      example = "#FF0000";
    };

  tomlConfig = {
    mpd = {
      address = cfg.settings.mpd.address;
      music_dir = cfg.settings.mpd.music_dir;
    };
    colors = {
      inherit (cfg.settings.colors) border title album artist status;
    };
  };
  configFile = pkgs.writeText "config.toml" (lib.generators.toTOML {} tomlConfig);
in {
  options.programs.zarumet = {
    enable = mkEnableOption "zarumet";
    package = mkOption {
      type = types.package;
      default = zarumet;
      description = "The zarumet package to use";
    };

    settings = {
      mpd = {
        address = mkOption {
          type = types.str;
          default = "localhost:6600";
          description = "MPD server address";
          example = "192.168.1.100:6600";
        };

        music_dir = mkOption {
          type = types.str;
          default = config.services.mpd.musicDirectory;
          description = "Path to your music directory";
          example = "/mnt/music";
        };
      };

      colors = {
        border = hexColorOption "#FAE280";
        title = hexColorOption "#FAE280";
        album = hexColorOption "#FAE280";
        artist = hexColorOption "#FAE280";
        status = hexColorOption "#FAE280";
      };
    };
  };
  config = mkIf cfg.enable {
    home.packages = [cfg.package];
    xdf.configFile."zarumet/config.toml".source = configFile;
  };
}
