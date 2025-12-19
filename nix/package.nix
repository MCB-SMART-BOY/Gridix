{ lib
, rustPlatform
, fetchFromGitHub
, pkg-config
, gtk3
, xdotool
, openssl
, stdenv
, darwin
}:

rustPlatform.buildRustPackage rec {
  pname = "gridix";
  version = "0.5.2";

  src = fetchFromGitHub {
    owner = "MCB-SMART-BOY";
    repo = "Gridix";
    rev = "v${version}";
    hash = "sha256-E44kvbXNj4+rqj6ahnR0eoV+VTys9t/f92hqQgdnZZc=";
  };

  cargoHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=";

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs = [
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

  meta = with lib; {
    description = "Fast, secure, cross-platform database management tool with Helix/Vim keybindings";
    longDescription = ''
      Gridix is a keyboard-driven database management tool supporting SQLite,
      PostgreSQL, and MySQL. Features include SSH tunneling, SSL/TLS encryption,
      19 built-in themes, and Helix/Vim-style keybindings.
    '';
    homepage = "https://github.com/MCB-SMART-BOY/Gridix";
    changelog = "https://github.com/MCB-SMART-BOY/Gridix/releases/tag/v${version}";
    license = licenses.mit;
    maintainers = with maintainers; [ ];
    mainProgram = "gridix";
    platforms = platforms.unix;
  };
}
