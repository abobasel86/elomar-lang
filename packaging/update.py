"""Writes the package-manager files for one release from its SHA256SUMS.txt:

- bucket/elomar.json             Scoop (Windows):  scoop bucket add elomar <repo>
- Formula/elomar.rb              Homebrew (macOS, Linux):  brew tap abobasel86/elomar <repo>
- packaging/winget/*.yaml        winget (Windows): sent to microsoft/winget-pkgs

    python3 packaging/update.py 1.12.1 path/to/SHA256SUMS.txt

The release workflow runs it after each release and commits the result to
main, so `scoop update` and `brew upgrade` find the new version.
"""
import json
import pathlib
import sys

REPO = "abobasel86/elomar-lang"
HOMEPAGE = "https://abobasel86.github.io/elomar-lang/"
DESC = "A statically typed programming language with AI built in, in English and Arabic"
ROOT = pathlib.Path(__file__).resolve().parent.parent
WINGET_ID = "ElomarLang.ElomarLang"


def main():
    version, sums_path = sys.argv[1], sys.argv[2]
    sums = {}
    for line in pathlib.Path(sums_path).read_text(encoding="utf-8").splitlines():
        if line.strip():
            digest, name = line.split()
            sums[name.lstrip("*")] = digest
    base = f"https://github.com/{REPO}/releases/download/v{version}"

    def asset(target, ext):
        name = f"elomar-{version}-{target}.{ext}"
        if name not in sums:
            sys.exit(f"{name} is not in {sums_path}")
        return f"{base}/{name}", sums[name], f"elomar-{version}-{target}"

    write("bucket/elomar.json", scoop(version, asset))
    write("Formula/elomar.rb", homebrew(version, asset))
    for name, text in winget(version, asset).items():
        write(f"packaging/winget/{name}", text)
    print(f"package files for {version} written")


def write(rel, text):
    path = ROOT / rel
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8", newline="\n")


def scoop(version, asset):
    arch = {}
    for key, target in [("64bit", "x86_64-pc-windows-msvc"), ("arm64", "aarch64-pc-windows-msvc")]:
        url, digest, folder = asset(target, "zip")
        arch[key] = {"url": url, "hash": digest, "extract_dir": folder}
    manifest = {
        "version": version,
        "description": DESC,
        "homepage": HOMEPAGE,
        "license": "MIT|Apache-2.0",
        "architecture": arch,
        "bin": "elomar.exe",
        "checkver": {"github": f"https://github.com/{REPO}"},
        "autoupdate": {
            "architecture": {
                "64bit": {
                    "url": f"https://github.com/{REPO}/releases/download/v$version/elomar-$version-x86_64-pc-windows-msvc.zip",
                    "extract_dir": "elomar-$version-x86_64-pc-windows-msvc",
                },
                "arm64": {
                    "url": f"https://github.com/{REPO}/releases/download/v$version/elomar-$version-aarch64-pc-windows-msvc.zip",
                    "extract_dir": "elomar-$version-aarch64-pc-windows-msvc",
                },
            },
            "hash": {"url": "$baseurl/SHA256SUMS.txt"},
        },
    }
    return json.dumps(manifest, indent=4) + "\n"


def homebrew(version, asset):
    def block(target):
        url, digest, _ = asset(target, "tar.gz")
        return f'      url "{url}"\n      sha256 "{digest}"'

    return f'''# Homebrew formula, written by packaging/update.py for each release.
#   brew tap abobasel86/elomar https://github.com/{REPO}
#   brew install elomar
class Elomar < Formula
  desc "Typed programming language with AI built in, in English and Arabic"
  homepage "{HOMEPAGE}"
  version "{version}"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    on_arm do
{block("aarch64-apple-darwin")}
    end
    on_intel do
{block("x86_64-apple-darwin")}
    end
  end

  on_linux do
    on_arm do
{block("aarch64-unknown-linux-gnu")}
    end
    on_intel do
{block("x86_64-unknown-linux-gnu")}
    end
  end

  def install
    bin.install "elomar"
  end

  test do
    assert_match version.to_s, shell_output("#{{bin}}/elomar --version")
    (testpath/"main.elo").write("fn main() {{\\n    print(1 + 2)\\n}}\\n")
    assert_equal "3\\n", shell_output("#{{bin}}/elomar run #{{testpath}}/main.elo")
  end
end
'''


def winget(version, asset):
    head = f"PackageIdentifier: {WINGET_ID}\nPackageVersion: {version}\n"
    tail = "ManifestVersion: 1.9.0\n"
    installers = ""
    for arch, target in [("x64", "x86_64-pc-windows-msvc"), ("arm64", "aarch64-pc-windows-msvc")]:
        url, digest, folder = asset(target, "zip")
        installers += f"""- Architecture: {arch}
  InstallerUrl: {url}
  InstallerSha256: {digest.upper()}
  NestedInstallerFiles:
  - RelativeFilePath: {folder}\\elomar.exe
    PortableCommandAlias: elomar
"""
    schema = "# yaml-language-server: $schema=https://aka.ms/winget-manifest.{}.1.9.0.schema.json\n"
    return {
        f"{WINGET_ID}.yaml": schema.format("version") + head
        + "DefaultLocale: en-US\nManifestType: version\n" + tail,
        f"{WINGET_ID}.installer.yaml": schema.format("installer") + head
        + "InstallerType: zip\nNestedInstallerType: portable\nCommands:\n- elomar\n"
        + "FileExtensions:\n- elo\nInstallers:\n" + installers
        + "ManifestType: installer\n" + tail,
        f"{WINGET_ID}.locale.en-US.yaml": schema.format("defaultLocale") + head
        + f"""PackageLocale: en-US
Publisher: Elomar-Lang
PublisherUrl: https://github.com/abobasel86
PublisherSupportUrl: https://github.com/{REPO}/issues
PackageName: Elomar-Lang
PackageUrl: {HOMEPAGE}
License: MIT OR Apache-2.0
LicenseUrl: https://github.com/{REPO}/blob/main/LICENSE-MIT
ShortDescription: {DESC}.
Description: |-
  Elomar-Lang is a small statically typed language that can be learned in Arabic as well as English:
  error messages in both languages, a full tutorial, and typed AI calls (OpenAI, Claude, or local
  models with Ollama) built into the language.
Moniker: elomar
Tags:
- programming-language
- arabic
- ai
- compiler
- interpreter
ReleaseNotesUrl: https://github.com/{REPO}/releases/tag/v{version}
ManifestType: defaultLocale
""" + tail,
    }


if __name__ == "__main__":
    main()
