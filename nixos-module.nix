{ localFlake, withSystem }:
{config, lib, pkgs, ...}:
with lib;
let cfg = config.services.capturebot;
in
{
  options.services.capturebot = {
    enable = mkEnableOption { description =  "enable the capturebot daemon"; };
    package = mkOption {
      default = withSystem ({ config, ... }: config.packages.default);
      defaultText = lib.literalMD "`packages.default` from the foo flake";
    };
    botToken = mkOption {
      type = types.str;
      default = null;
      description = "Telegram Bot token for capturebot.";
    };
    userId = mkOption {
      type = types.int;
      default = null;
      description = "User ID that is going to be talking to capturebot.";
    };
    saveDir = mkOption {
      type = types.path;
      default = null;
      description = "Path capturebot saves notes to";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.capturebot = {
      name = "capturebot";
      wantedBy = "network-online.target";
      serviceConfig = {
        ExecStart = "${cfg.package}/bin/capturebot";
        RestartSec = 3;
        Restart = "always";
        RestartSteps = 3;
      };
      environment = {
        "CAPTUREBOT_USER_ID" = cfg.userId;
        "CAPTUREBOT_SAVE_DIR" = cfg.SaveDir;
        "TELOXIDE_TOKEN" = cfg.botToken;
      };
    };
  };
}
