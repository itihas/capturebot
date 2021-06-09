{
  outputs = { self, nixpkgs }: {
    defaultPackage."x86_64-linux" = ./default.nix;
    nixosModules = {
      service = ./capturebot.nix;   
    };
  };
}
