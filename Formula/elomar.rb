# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "https://abobasel86.github.io/elomar-lang/"
  version "1.20.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.20.0/elomar-1.20.0-aarch64-apple-darwin.tar.gz"
      sha256 "5bbe471c22dd5245dced344d27f42eedd51f1cfac5ffa5ba4c5e16cc4999425c"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.20.0/elomar-1.20.0-x86_64-apple-darwin.tar.gz"
      sha256 "2bdc86c4f31de6cd76e92613f3070ee4af0b67c46ae5e8298e097a0a8db5be4b"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.20.0/elomar-1.20.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "69976279957ca69ed3a04e10c075068d6ce11bf271b5dd1a76b247d43f2faea1"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.20.0/elomar-1.20.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "860233919fc1e809689d6377c888ab565e4a0ac80cd9a60988b7ab1420adaed6"
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
