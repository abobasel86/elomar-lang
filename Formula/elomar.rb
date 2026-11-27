# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "https://abobasel86.github.io/elomar-lang/"
  version "1.17.0"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.17.0/elomar-1.17.0-aarch64-apple-darwin.tar.gz"
      sha256 "f17a80f585c5c964cbdcad597823347da3015f2353799d4e17d0ae1edd20ad5e"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.17.0/elomar-1.17.0-x86_64-apple-darwin.tar.gz"
      sha256 "95db50e89e92ab11afbc8219e71ba0f28babc3e53f664d7fca1b5673b5a77236"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.17.0/elomar-1.17.0-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "970076b674262dae931872716759e03a358f4bece0dd4e11cf7cfa1b28a84d33"
    end
    on_intel do
      url "https://github.com/abobasel86/elomar-lang/releases/download/v1.17.0/elomar-1.17.0-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "a54bb9bc57e0c943bff628d18a85eafb6a961f5bf27d33e81835f47b27d5f440"
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
