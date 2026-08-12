# Homebrew Formula for Gridix
# Repository: https://github.com/MCB-SMART-BOY/homebrew-gridix

class Gridix < Formula
  desc "Fast, secure database management tool with Helix/Vim keybindings"
  homepage "https://github.com/MCB-SMART-BOY/Gridix"
  version "7.2.0"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v7.2.0/gridix-macos-arm64.tar.gz"
      sha256 "8ff8e2c1c7b294430e6dbd32462230b48379c5a2f3a194a204ab6ce51ab6cae0"
    end
  end

  on_linux do
    url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v7.2.0/gridix-linux-x86_64.tar.gz"
    sha256 "accbe6ff469f47e4699061eca402b78ed981fae8d1c31dd07500791cae722df6"
  end

  def install
    bin.install "gridix"
  end

  test do
    system "#{bin}/gridix", "--help"
  end
end
