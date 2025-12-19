{
  lib,
  rustPlatform,
  fetchFromGitHub,
  pkg-config,
  gtk3,
  xdotool,
  openssl,
  stdenv,
  darwin,
}:

rustPlatform.buildRustPackage (finalAttrs: {
  pname = "gridix";
  version = "0.5.2";

  src = fetchFromGitHub {
    owner = "MCB-SMART-BOY";
    repo = "Gridix";
    tag = "v${finalAttrs.version}";
    hash = "sha256-7AvaSqLjDAA65V5UXZRd6Gjgns4smJnI01T14RoQqBg=";
  };

  cargoHash = "sha256-EW6fVDYD2Fsw8hNhwN/M9tT53ivRl1uc/JqQMgIH57k=";

  nativeBuildInputs = [
    pkg-config
  ];

  buildInputs =
    [
      gtk3
      xdotool
      openssl
    ]
    ++ lib.optionals stdenv.isDarwin [
      darwin.apple_sdk.frameworks.AppKit
      darwin.apple_sdk.frameworks.CoreGraphics
      darwin.apple_sdk.frameworks.CoreText
      darwin.apple_sdk.frameworks.Foundation
      darwin.apple_sdk.frameworks.Metal
      darwin.apple_sdk.frameworks.QuartzCore
    ];

  meta = {
    description = "Fast, secure, cross-platform database management tool with Helix/Vim keybindings";
    longDescription = ''
      Gridix is a keyboard-driven database management tool supporting SQLite,
      PostgreSQL, and MySQL. Features include SSH tunneling, SSL/TLS encryption,
      19 built-in themes, and Helix/Vim-style keybindings.
    '';
    homepage = "https://github.com/MCB-SMART-BOY/Gridix";
    changelog = "https://github.com/MCB-SMART-BOY/Gridix/releases/tag/v${finalAttrs.version}";
    license = lib.licenses.mit;
    maintainers = with lib.maintainers; [ mcbgaruda ];
    mainProgram = "gridix";
    platforms = lib.platforms.unix;
  };
})
