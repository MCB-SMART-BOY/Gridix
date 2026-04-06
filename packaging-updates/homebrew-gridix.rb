# Homebrew Formula for Gridix
# Repository: https://github.com/MCB-SMART-BOY/homebrew-gridix

class Gridix < Formula
  desc "Fast, secure database management tool with Helix/Vim keybindings"
  homepage "https://github.com/MCB-SMART-BOY/Gridix"
  version "3.7.1"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.7.1/gridix-macos-arm64.tar.gz"
      sha256 "5c033e620863f6eda0f0dd34887727237907c43e389b6033c8c4441945504f50"
    end
    on_intel do
      # Intel Mac users can use ARM build with Rosetta 2
      url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.7.1/gridix-macos-arm64.tar.gz"
      sha256 "5c033e620863f6eda0f0dd34887727237907c43e389b6033c8c4441945504f50"
    end
  end

  on_linux do
    url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.7.1/gridix-linux-x86_64.tar.gz"
    sha256 "2f12ab1224617ea660582a81831799b1be10de3917087884f8366c3f3cbf72b1"
  end

  def install
    bin.install "gridix"
  end

  test do
    system "#{bin}/gridix", "--help"
  end
end
