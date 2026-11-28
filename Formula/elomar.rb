# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "https://abobasel86.github.io/elomar-lang/"
  version "1.19.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.19.0/elomar-1.19.0-aarch64-apple-darwin.tar.gz"
      sha256 "ced455b9228d94472d134a8839c9d2618119f5adf0c436cc58057248c33b6e53"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.19.0/elomar-1.19.0-x86_64-apple-darwin.tar.gz"
      sha256 "4ec8b81520eb04521f7ad76d18f5b954e72841a167671d4e9137067a17ba681a"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.19.0/elomar-1.19.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "67f431993607f3111d9b66ebd91957e5f9e4f5b262141cef5e0317b4be5dbc91"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.19.0/elomar-1.19.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "056a6a1091b817ff49af3b6146f095eca5f925cc2dba464480084f89d21c9a7a"
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
