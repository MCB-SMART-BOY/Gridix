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
        cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
        commonBuildInputs = with pkgs; [
          openssl
        ];
        linuxRuntimeLibs = with pkgs; [
          gtk3
          xdotool
          wayland
          libxkbcommon
          libglvnd
          mesa
        ];
        runtimeLibraryPath = pkgs.lib.makeLibraryPath linuxRuntimeLibs;
        gridixPackage = pkgs.rustPlatform.buildRustPackage rec {
          pname = "gridix";
          version = cargoToml.package.version;

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
          ];

          buildInputs =
            commonBuildInputs
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux linuxRuntimeLibs
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
            license = licenses.asl20;
            maintainers = [{
              name = "MCB-SMART-BOY";
              email = "mcb2720838051@gmail.com";
              github = "MCB-SMART-BOY";
            }];
            mainProgram = "gridix";
            platforms = platforms.linux ++ platforms.darwin;
          };
        };
      in
      {
        packages = {
          default = gridixPackage;
          gridix = gridixPackage;
        };

        apps = {
          default = {
            type = "app";
            program = "${gridixPackage}/bin/gridix";
            meta = {
              description = "Launch Gridix from flake app output";
            };
          };
          gridix = {
            type = "app";
            program = "${gridixPackage}/bin/gridix";
            meta = {
              description = "Launch Gridix from flake app output";
            };
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs =
            with pkgs;
            [
              rustup
              pkg-config
              openssl
            ]
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux linuxRuntimeLibs
            ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
              pkgs.darwin.apple_sdk.frameworks.AppKit
              pkgs.darwin.apple_sdk.frameworks.CoreGraphics
              pkgs.darwin.apple_sdk.frameworks.CoreText
              pkgs.darwin.apple_sdk.frameworks.Foundation
              pkgs.darwin.apple_sdk.frameworks.Metal
              pkgs.darwin.apple_sdk.frameworks.QuartzCore
            ];

          shellHook = pkgs.lib.optionalString pkgs.stdenv.isLinux ''
            export LD_LIBRARY_PATH="${runtimeLibraryPath}:''${LD_LIBRARY_PATH:-}"
            export __EGL_VENDOR_LIBRARY_DIRS="${pkgs.mesa}/share/glvnd/egl_vendor.d"
            export LIBGL_DRIVERS_PATH="${pkgs.mesa}/lib/dri"
          '';
        };
      }
    )
    // {
      overlays.default = final: prev: {
        gridix = self.packages.${prev.stdenv.hostPlatform.system}.default;
      };
    };
}
