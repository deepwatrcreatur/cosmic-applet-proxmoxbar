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

          # Use fetchCargoVendor instead of allowBuiltinFetchGit to avoid
          # SSH key issues when fetching git dependencies (works for all users)
          useFetchCargoVendor = true;
          cargoHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

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
