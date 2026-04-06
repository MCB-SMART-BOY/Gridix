# Homebrew Formula for Gridix
# Repository: https://github.com/MCB-SMART-BOY/homebrew-gridix

class Gridix < Formula
  desc "Fast, secure database management tool with Helix/Vim keybindings"
  homepage "https://github.com/MCB-SMART-BOY/Gridix"
  version "3.7.0"
  license "Apache-2.0"

  on_macos do
    on_arm do
      url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.7.0/gridix-macos-arm64.tar.gz"
      sha256 "e9af7034577ee9e004caaf716a8cdbb4a8dd2fb0e3fbacddcc7043077e345635"
    end
    on_intel do
      # Intel Mac users can use ARM build with Rosetta 2
      url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.7.0/gridix-macos-arm64.tar.gz"
      sha256 "e9af7034577ee9e004caaf716a8cdbb4a8dd2fb0e3fbacddcc7043077e345635"
    end
  end

  on_linux do
    url "https://github.com/MCB-SMART-BOY/Gridix/releases/download/v3.7.0/gridix-linux-x86_64.tar.gz"
    sha256 "2bb3ac1c8fc67ace583eae5133669abd02601b232749f497029395ee0ccb0b17"
  end

  def install
    bin.install "gridix"
  end

  test do
    system "#{bin}/gridix", "--help"
  end
end
