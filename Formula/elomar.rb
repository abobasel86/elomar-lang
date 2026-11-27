# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "https://abobasel86.github.io/elomar-lang/"
  version "1.16.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.16.0/elomar-1.16.0-aarch64-apple-darwin.tar.gz"
      sha256 "75654134358121b8242bfa01c9b149ce062a0ea441082d84a97ea025089a7196"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.16.0/elomar-1.16.0-x86_64-apple-darwin.tar.gz"
      sha256 "7df932bcf696744b178768225d434f6df4adaec27888a0df8a954aa71046afe6"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.16.0/elomar-1.16.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "21349eca9b72567939d752d461c3efd8ae69b8f93c39b16a05832a7e8e3df2e5"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.16.0/elomar-1.16.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "c0ad5c20c2d34682bbe4d17f27921e79b3f5a54c713e0fdc98f9d0e65c5dd9b6"
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
