#!/bin/sh
# Installs Elomar-Lang on Linux and macOS:
#
#   curl -fsSL https://raw.githubusercontent.com/abobasel86/elomar-lang/main/install.sh | sh
#
# It downloads the latest release for this system, checks it against the
# release's SHA256SUMS.txt, puts `elomar` in ~/.elomar/bin and adds that
# folder to PATH in your shell's startup file. Running it again upgrades.
#
# Settings (environment variables, all optional):
#   ELOMAR_VERSION      a version such as 1.11.0 (default: the latest)
#   ELOMAR_INSTALL_DIR  where to put `elomar`
#   ELOMAR_NO_PATH=1    do not change any startup file
#   ELOMAR_TARGET       the build to download (default: this system's)

set -eu

repo="abobasel86/elomar-lang"

fail() {
    echo "error: $*" >&2
    exit 1
}

if [ -n "${ELOMAR_TARGET:-}" ]; then
    target="$ELOMAR_TARGET"
else
    case "$(uname -s)-$(uname -m)" in
        Linux-x86_64 | Linux-amd64) target="x86_64-unknown-linux-gnu" ;;
        Darwin-arm64 | Darwin-aarch64) target="aarch64-apple-darwin" ;;
        Darwin-x86_64) target="x86_64-apple-darwin" ;;
        *)
            fail "there is no ready-made Elomar-Lang for $(uname -s) $(uname -m).
Build it from source instead: cargo install --git https://github.com/$repo elomar-cli --locked"
            ;;
    esac
fi

if command -v curl > /dev/null 2>&1; then
    fetch() { curl -fsSL --proto '=https' --tlsv1.2 "$1" -o "$2"; }
elif command -v wget > /dev/null 2>&1; then
    fetch() { wget -q --https-only "$1" -O "$2"; }
else
    fail "curl or wget is needed to download Elomar-Lang"
fi

if command -v sha256sum > /dev/null 2>&1; then
    sha256() { sha256sum "$1" | cut -d' ' -f1; }
elif command -v shasum > /dev/null 2>&1; then
    sha256() { shasum -a 256 "$1" | cut -d' ' -f1; }
else
    fail "sha256sum or shasum is needed to check the download"
fi

tmp="$(mktemp -d 2> /dev/null || mktemp -d -t elomar)"
trap 'rm -rf "$tmp"' EXIT INT TERM

if [ -n "${ELOMAR_VERSION:-}" ]; then
    tag="v${ELOMAR_VERSION#v}"
else
    fetch "https://api.github.com/repos/$repo/releases/latest" "$tmp/latest.json" ||
        fail "could not ask GitHub for the latest release"
    tag="$(sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$tmp/latest.json" | head -n 1)"
    [ -n "$tag" ] || fail "could not read the latest release's version"
fi
version="${tag#v}"
name="elomar-$version-$target"
base="https://github.com/$repo/releases/download/$tag"
dir="${ELOMAR_INSTALL_DIR:-$HOME/.elomar/bin}"

echo "Downloading Elomar-Lang $version ($target) ..."
fetch "$base/$name.tar.gz" "$tmp/$name.tar.gz" || fail "could not download $name.tar.gz"
fetch "$base/SHA256SUMS.txt" "$tmp/SHA256SUMS.txt" || fail "could not download SHA256SUMS.txt"

expected="$(awk -v f="$name.tar.gz" '{ n = $2; sub(/^\*/, "", n); if (n == f) print $1 }' "$tmp/SHA256SUMS.txt" | head -n 1)"
[ -n "$expected" ] || fail "SHA256SUMS.txt has no line for $name.tar.gz"
actual="$(sha256 "$tmp/$name.tar.gz")"
[ "$expected" = "$actual" ] || fail "checksum mismatch for $name.tar.gz: the download is damaged"

tar -xzf "$tmp/$name.tar.gz" -C "$tmp"
mkdir -p "$dir"
# Replacing the file (not writing into it) is safe even while it runs.
cp "$tmp/$name/elomar" "$dir/elomar.new"
chmod 755 "$dir/elomar.new"
mv -f "$dir/elomar.new" "$dir/elomar"

if [ "${ELOMAR_NO_PATH:-}" != "1" ]; then
    case ":$PATH:" in
        *":$dir:"*) ;;
        *)
            case "$(basename "${SHELL:-sh}")" in
                zsh) profile="$HOME/.zshrc" ;;
                bash)
                    if [ "$(uname -s)" = "Darwin" ]; then profile="$HOME/.bash_profile"; else profile="$HOME/.bashrc"; fi
                    ;;
                fish) profile="" ;;
                *) profile="$HOME/.profile" ;;
            esac
            if [ -n "$profile" ]; then
                line="export PATH=\"$dir:\$PATH\""
                if ! grep -qsF "$line" "$profile"; then
                    printf '\n# Elomar-Lang\n%s\n' "$line" >> "$profile"
                    echo "Added $dir to PATH in $profile (open a new terminal to use it)."
                fi
            else
                echo "Add $dir to your PATH: fish_add_path $dir"
            fi
            ;;
    esac
fi

if [ -z "${ELOMAR_TARGET:-}" ]; then
    "$dir/elomar" --version
fi
echo "Elomar-Lang $version is installed: $dir/elomar"
echo "Start with:  elomar new hello   (or: elomar new helpdesk --ai mock)"
