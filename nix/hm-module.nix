self: {
  lib,
  pkgs,
  config,
  ...
}:
with lib; let
  cfg = config.programs.zarumet;
  zarumet = self.packages.${pkgs.stdenv.hostPlatform.system}.default;

  tomlFormat = pkgs.formats.toml {};
in {
  options.programs.zarumet = {
    enable = mkEnableOption "zarumet";
    package = mkOption {
      type = types.package;
      default = zarumet;
      description = "The zarumet package to use";
    };

    settings = mkOption {
      type = tomlFormat.type;
      default = {};
      example = literalExpression ''
        {
          address = "localhost:6600";
          music_dir = "/mnt/music";
        }
      '';
      description = "Settings for zarumet";
    };
  };
  config = mkIf cfg.enable {
    home.packages = mkIf (cfg.package != null) [cfg.package];

    xdf.configFile."zarumet/config.toml" = mkIf (cfg.settings != {}) {
      source = tomlFormat.generate "config.toml" cfg.settings;
    };
  };
}
