# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "https://abobasel86.github.io/elomar-lang/"
  version "1.15.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.15.0/elomar-1.15.0-aarch64-apple-darwin.tar.gz"
      sha256 "27c0d3267265848fba2c75fcdeafc7f4d166f2e5df782fbbc7f66307f9760836"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.15.0/elomar-1.15.0-x86_64-apple-darwin.tar.gz"
      sha256 "8ecf5a7b07222701c19f72c8df3cd7a6d1b765c5bd5ad94151cc0843a643d875"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.15.0/elomar-1.15.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "d816e407babe23dbd107e7e86c519ed85a88a3ff9e0143a9d25ed3fa5ce2d1eb"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.15.0/elomar-1.15.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "20dcf653f0da29dcefb7f6da62abd84b507e23b7fed4103e9c123d161002edcf"
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
