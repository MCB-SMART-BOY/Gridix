{
  description = "Gridix - Fast, secure database management tool with Helix/Vim keybindings";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
      {
        packages.default = pkgs.rustPlatform.buildRustPackage rec {
          pname = "gridix";
          version = "2.0.0";

          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
          ];

          buildInputs = with pkgs; [
            gtk3
            xdotool
            openssl
          ] ++ lib.optionals stdenv.isDarwin [
            darwin.apple_sdk.frameworks.AppKit
            darwin.apple_sdk.frameworks.CoreGraphics
            darwin.apple_sdk.frameworks.CoreText
            darwin.apple_sdk.frameworks.Foundation
            darwin.apple_sdk.frameworks.Metal
            darwin.apple_sdk.frameworks.QuartzCore
          ];

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
            platforms = platforms.unix;
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            cargo
            rustc
            rust-analyzer
            pkg-config
            gtk3
            xdotool
            openssl
          ];
        };
      }
    );
}
