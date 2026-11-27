# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "https://abobasel86.github.io/elomar-lang/"
  version "1.18.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.18.0/elomar-1.18.0-aarch64-apple-darwin.tar.gz"
      sha256 "f49274c6559bae13bf20fa59202fbfaa4dbc9963fb996e8e623203cf23eb82a6"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.18.0/elomar-1.18.0-x86_64-apple-darwin.tar.gz"
      sha256 "0df3665a531ebd3c928d723f09fb7a37a2f75e58ea66d6c17a43a35b791b7f6a"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.18.0/elomar-1.18.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "c6dea74a3d8941ff671b9b24f1938baa590c5a3ce5318f78323f021522fa0241"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.18.0/elomar-1.18.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "c0c34b99d53c9136baec49175fdb010569719607ff5a6fa83b49ee85e070f0d2"
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
