# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "https://abobasel86.github.io/elomar-lang/"
  version "1.14.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.14.0/elomar-1.14.0-aarch64-apple-darwin.tar.gz"
      sha256 "e4d2c37493a92a24a38466541f58f55fe63ae37726c7aaf71e0c372218da12d3"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.14.0/elomar-1.14.0-x86_64-apple-darwin.tar.gz"
      sha256 "b91ab5d1f175fbe975c158518999161bd896bee5cebabb212900751601fb821b"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.14.0/elomar-1.14.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "2c9a63ed27c49f17ee63538bac483e1c2f96f7407901016833e47271f4793584"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.14.0/elomar-1.14.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "bd83dd79f5c24b611f6e7c99a98550041541166a2f2b48d3bcba79545285cec8"
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
