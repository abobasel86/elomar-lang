# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "https://abobasel86.github.io/elomar-lang/"
  version "1.13.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.13.0/elomar-1.13.0-aarch64-apple-darwin.tar.gz"
      sha256 "e4e490d749a23ae63aeb164497777cc07bdfcf66a80e1cb92ce61244af6c9df2"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.13.0/elomar-1.13.0-x86_64-apple-darwin.tar.gz"
      sha256 "8836011e866605bda2bbc2d4b0ed1e4bc9179c6555b86a6b7b0cafd5a9dc33ce"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.13.0/elomar-1.13.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "1f5bc53e24c71663ffe5a9490d2235a3403a24e5fb195edca813bcb005ea5d46"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.13.0/elomar-1.13.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "6b8eeddc665f28f1b3520d747743cc504eee3e0ae2a0d63170320d8b559037d9"
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
