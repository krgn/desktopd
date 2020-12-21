let
  pkgs = import <nixpkgs> {};
  secrets = import ../secrets.nix;
in
  pkgs.mkShell {
    RUST_LOG = "info";
    API_KEY = secrets.MOZ_API_KEY;
    API_SECRET = secrets.MOZ_API_SECRET;
    buildInputs = with pkgs; [
      pkgconfig openssl ws dbus
    ];
  }
