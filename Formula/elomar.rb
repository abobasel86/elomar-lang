# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "https://abobasel86.github.io/elomar-lang/"
  version "1.12.1"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.12.1/elomar-1.12.1-aarch64-apple-darwin.tar.gz"
      sha256 "dbc0de3fa839e1a32c52d828a025867315a6813021e1307eff6701641c295310"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.12.1/elomar-1.12.1-x86_64-apple-darwin.tar.gz"
      sha256 "65b7fc13364de17bdbe0c20ece7e6ab3f4a0f6286db4153baa66f45efe4995e3"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.12.1/elomar-1.12.1-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "868863161dc512a188b1804478093790e5d0c925846d2ceb6fae5431900ed8f2"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.12.1/elomar-1.12.1-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "d8d1a497b0167f6b4264120d987629fcf930ec76c3c48a0e4fbf0ce37bc03078"
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
