{ lib, config, pkgs,... }:

let
  capturebot = pkgs.callPackage /home/sahiti/bucket/capturebot {};
in
{

  
  systemd.user.services.capturebot = {
    path = [ capturebot pkgs.xdg_utils pkgs.emacs ];
    script = "${capturebot}/bin/capturebot";
    serviceConfig = {
      Type = "simple";
    };
    enable = true;
    description = "org capture bot";
  };

}
