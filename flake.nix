{
  inputs.mach-nix.url = "github:DavHau/mach-nix";
  outputs = { self, nixpkgs, mach-nix }:
    let
      system = "x86_64-linux";
    in
      rec {
        defaultPackage.${system} = mach-nix.lib.${system}.buildPythonPackage {
          pname = "capturebot";
          version = "0.1";
          src = ./. ;
          requirements = ''python-telegram-bot'';

        };

        nixosModules = {
          service = { pkgs,...}:
            {
              systemd.user.services.capturebot = {
                path = [ defaultPackage.${system} pkgs.xdg_utils pkgs.emacs ];
                script = "capturebot";
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
