# shell.nix
#
# This file is a development requirement and must not be moved. Thanks.
let
  pkgs = import <nixpkgs> {};
in
  pkgs.mkShell {
    buildInputs = with pkgs; [
      pkgconfig openssl
    ];
  }
