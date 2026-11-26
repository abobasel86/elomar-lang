# Package managers

`update.py` writes these files from a release's `SHA256SUMS.txt`:

| Manager | File | Users run |
|---|---|---|
| Scoop (Windows) | `bucket/elomar.json` | `scoop bucket add elomar https://github.com/abobasel86/elomar-lang` then `scoop install elomar` |
| Homebrew (macOS, Linux) | `Formula/elomar.rb` | `brew tap abobasel86/elomar https://github.com/abobasel86/elomar-lang` then `brew install elomar` |
| winget (Windows) | `packaging/winget/*.yaml` | `winget install ElomarLang.ElomarLang`, once Microsoft accepts it |

The release workflow (`.github/workflows/release.yml`) runs `update.py`
after each release and commits the result to `main`, so `scoop update` and
`brew upgrade` see the new version. CI installs the latest release with
Scoop and Homebrew from these files. By hand:

```bash
gh release download v1.12.1 -p SHA256SUMS.txt
python3 packaging/update.py 1.12.1 SHA256SUMS.txt
```

## winget

winget's packages live in [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs),
so each version is a pull request there, sent from the maintainer's own
GitHub account. With [wingetcreate](https://github.com/microsoft/winget-create)
(`winget install wingetcreate`), which signs in to GitHub in the browser:

```powershell
winget validate --manifest packaging\winget
wingetcreate submit packaging\winget
```

For later versions, after the release workflow has updated the files:

```powershell
wingetcreate update ElomarLang.ElomarLang --version 1.12.2 --urls `
  https://github.com/abobasel86/elomar-lang/releases/download/v1.12.2/elomar-1.12.2-x86_64-pc-windows-msvc.zip|x64 `
  https://github.com/abobasel86/elomar-lang/releases/download/v1.12.2/elomar-1.12.2-aarch64-pc-windows-msvc.zip|arm64 `
  --submit
```
