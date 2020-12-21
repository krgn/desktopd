let
  pkgs = import <nixpkgs> {};
in
  pkgs.mkShell {
    RUST_LOG = "info";
    buildInputs = with pkgs; [
      pkgconfig openssl ws dbus
    ];
  }
