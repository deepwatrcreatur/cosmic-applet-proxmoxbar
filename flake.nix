{
  description = "COSMIC applet for Proxmox status";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage {
          pname = "cosmic-applet-proxmoxbar";
          version = "0.1.0";
          src = ./.;
          cargoLock = {
            lockFile = ./Cargo.lock;
            allowBuiltinFetchGit = true;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            fontconfig
            libxkbcommon
            wayland
          ];

          postInstall = ''
            mkdir -p $out/share/applications
            cp data/com.deepwatrcreatur.CosmicAppletProxmoxbar.desktop $out/share/applications/
          '';
        };

        apps.default = {
          type = "app";
          program = "${self.packages.${system}.default}/bin/cosmic-applet-proxmoxbar";
        };
      }
    );
}
