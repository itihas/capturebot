{
  inputs.mach-nix.url = "github:DavHau/mach-nix";
  outputs = { self, nixpkgs, mach-nix }:
    let
      system = "x86_64-linux";
    in
      rec {
        defaultPackage.${system} = mach-nix.lib.${system}.buildPythonPackage {
          pname = "capturebot";
          version = "0.1.1rc1";
          src = ./. ;

        };

        nixosModules = {
          service = { config, pkgs, lib, ...}:
            {
              options.capturebot = {
                tokenFile = lib.mkOption {
                  type = lib.types.str;
                  default = "";
                };
                user = lib.mkOption {
                  type = lib.types.str;
                  default = "";
                };
              };

              config.systemd.user.services.capturebot = {
                path = [ defaultPackage.${system} pkgs.xdg_utils pkgs.emacs ];
                script = "capturebot --tokenfile ${config.capturebot.tokenFile} --user ${config.capturebot.user}";
                serviceConfig = {
                  Type = "exec";
                };
                enable = true;
                description = "org capture bot";
              };
            };
        };
      };
}
