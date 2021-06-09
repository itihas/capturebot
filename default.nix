{pkgs}:

with pkgs;

python38.pkgs.buildPythonApplication rec {
  pname = "capturebot";
  version = "0.1";
  src = ./. ;
  propagatedBuildInputs = [ python38.pkgs.python-telegram-bot ];

}
    
  
