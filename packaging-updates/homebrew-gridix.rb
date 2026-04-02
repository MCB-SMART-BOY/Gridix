# Homebrew Formula for Gridix
# Repository: https://github.com/MCB-SMART-BOY/homebrew-gridix

class Gridix < Formula
  desc "Fast, secure database management tool with Helix/Vim keybindings"
  homepage "https://github.com/MCB-SMART-BOY/Gridix"
  version "3.6.0"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.6.0/gridix-macos-arm64.tar.gz"
      sha256 "8f0d95ac8a8bf3a122832995b131038fe7237f1640eecfcf954e3512d0ca3684"
    end
    on_intel do
      # Intel Mac users can use ARM build with Rosetta 2
      url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.6.0/gridix-macos-arm64.tar.gz"
      sha256 "8f0d95ac8a8bf3a122832995b131038fe7237f1640eecfcf954e3512d0ca3684"
    end
  end

  on_linux do
    url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.6.0/gridix-linux-x86_64.tar.gz"
    sha256 "8d317ef03ae398a91b0bf5bd426470aed4d56d65c029211717b813f3bbe800c6"
  end

  def install
    bin.install "gridix"
  end

  test do
    system "#{bin}/gridix", "--help"
  end
end
