{
  description = "Gridix - Fast, secure database management tool with Helix/Vim keybindings";

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
        pkgs = nixpkgs.legacyPackages.${system};
        runtimeLibs = with pkgs; [
          gtk3
          xdotool
          openssl
          wayland
          libxkbcommon
          libglvnd
          mesa
        ];
        runtimeLibraryPath = pkgs.lib.makeLibraryPath runtimeLibs;
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage rec {
          pname = "gridix";
          version = "2.0.1";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
          ];

          buildInputs =
            runtimeLibs
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.AppKit
              pkgs.darwin.apple_sdk.frameworks.CoreGraphics
              pkgs.darwin.apple_sdk.frameworks.CoreText
              pkgs.darwin.apple_sdk.frameworks.Foundation
              pkgs.darwin.apple_sdk.frameworks.Metal
              pkgs.darwin.apple_sdk.frameworks.QuartzCore
            ];

          postFixup = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            wrapProgram "$out/bin/gridix" \
              --prefix LD_LIBRARY_PATH : "${runtimeLibraryPath}" \
              --set-default __EGL_VENDOR_LIBRARY_DIRS "${pkgs.mesa}/share/glvnd/egl_vendor.d" \
              --set-default LIBGL_DRIVERS_PATH "${pkgs.mesa}/lib/dri"
          '';

          meta = with pkgs.lib; {
            description = "Fast, secure, cross-platform database management tool with Helix/Vim keybindings";
            homepage = "https://github.com/MCB-SMART-BOY/Gridix";
            license = licenses.mit;
            maintainers = [ ];
            platforms = platforms.unix;
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs =
            with pkgs;
            [
              rustup
              pkg-config
            ]
            ++ runtimeLibs;

          shellHook = ''
            export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"
            export __EGL_VENDOR_LIBRARY_DIRS="${pkgs.mesa}/share/glvnd/egl_vendor.d"
            export LIBGL_DRIVERS_PATH="${pkgs.mesa}/lib/dri"
          '';
        };
      }
    );
}
