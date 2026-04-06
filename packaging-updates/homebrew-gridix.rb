# Homebrew Formula for Gridix
# Repository: https://github.com/MCB-SMART-BOY/homebrew-gridix

class Gridix < Formula
  desc "Fast, secure database management tool with Helix/Vim keybindings"
  homepage "https://github.com/MCB-SMART-BOY/Gridix"
  version "3.8.0"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.8.0/gridix-macos-arm64.tar.gz"
      sha256 "90bc6c61abb60a6c2985c69cecc386f0fa266b7e621ef196f82448ec0deec1cd"
    end
    on_intel do
      # Intel Mac users can use ARM build with Rosetta 2
      url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.8.0/gridix-macos-arm64.tar.gz"
      sha256 "90bc6c61abb60a6c2985c69cecc386f0fa266b7e621ef196f82448ec0deec1cd"
    end
  end

  on_linux do
    url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.8.0/gridix-linux-x86_64.tar.gz"
    sha256 "4232deb85430fa587971aec937da10ff00175a161c7cad30838f852f47dc566a"
  end

  def install
    bin.install "gridix"
  end

  test do
    system "#{bin}/gridix", "--help"
  end
end
