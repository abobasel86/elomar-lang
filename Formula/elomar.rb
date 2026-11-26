# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "https://abobasel86.github.io/elomar-lang/"
  version "1.13.1"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.13.1/elomar-1.13.1-aarch64-apple-darwin.tar.gz"
      sha256 "6a10274203fe16a513af43a65cf52e461faf219ed35fefd41c070c4687457866"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.13.1/elomar-1.13.1-x86_64-apple-darwin.tar.gz"
      sha256 "7ca17a3f14e6a134f785e43dc55d47121da9a170a6f3b914ac37c3cf6a2a9ebe"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.13.1/elomar-1.13.1-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "eb4f61495568a69c9305133aaf166ca5e0dc7f5bf6dbade0b902f92a72301e04"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.13.1/elomar-1.13.1-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "7b0897e6cf8cff94b2df85c332fffc38fd983fa86856ad4e6c87915cc4137701"
    end
  end

  def install
    bin.install "elomar"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/elomar --version")
    (testpath/"main.elo").write("fn main() {\n    print(1 + 2)\n}\n")
    assert_equal "3\n", shell_output("#{bin}/elomar run #{testpath}/main.elo")
  end
end
