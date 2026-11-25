# Installs Elomar-Lang on Windows:
#
#   irm https://raw.githubusercontent.com/abobasel86/elomar-lang/main/install.ps1 | iex
#
# It downloads the latest release for Windows (x64), checks it against the
# release's SHA256SUMS.txt, puts elomar.exe in %USERPROFILE%\.elomar\bin and
# adds that folder to your PATH. Running it again upgrades Elomar-Lang.
#
# Settings (environment variables, all optional):
#   ELOMAR_VERSION      a version such as 1.11.0 (default: the latest)
#   ELOMAR_INSTALL_DIR  where to put elomar.exe
#   ELOMAR_NO_PATH=1    do not change PATH

& {
    $ErrorActionPreference = 'Stop'
    $ProgressPreference = 'SilentlyContinue'
    # Windows PowerShell 5 may not enable TLS 1.2 by itself.
    [Net.ServicePointManager]::SecurityProtocol =
        [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12

    $repo = 'abobasel86/elomar-lang'
    $target = 'x86_64-pc-windows-msvc'
    if ($env:PROCESSOR_ARCHITECTURE -eq 'ARM64') {
        Write-Host 'Note: there is no ARM64 build yet; the x64 build runs under emulation.'
    }

    if ($env:ELOMAR_VERSION) {
        $tag = 'v' + $env:ELOMAR_VERSION.TrimStart('v')
    } else {
        # github.com/.../releases/latest redirects to .../releases/tag/vX.Y.Z.
        # Unlike the API, it has no low limit per address (shared networks).
        $tag = $null
        try {
            $request = [Net.HttpWebRequest]::Create("https://github.com/$repo/releases/latest")
            $request.Method = 'HEAD'
            $request.AllowAutoRedirect = $false
            $request.UserAgent = 'elomar-installer'
            $response = $request.GetResponse()
            $location = $response.Headers['Location']
            $response.Close()
            if ($location -match '/releases/tag/(v[^/]+)$') { $tag = $Matches[1] }
        } catch { }
        if (-not $tag) {
            $latest = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" `
                -Headers @{ 'User-Agent' = 'elomar-installer' }
            $tag = $latest.tag_name
        }
    }
    $version = $tag.TrimStart('v')
    $name = "elomar-$version-$target"
    $base = "https://github.com/$repo/releases/download/$tag"
    $dir = if ($env:ELOMAR_INSTALL_DIR) { $env:ELOMAR_INSTALL_DIR } else { Join-Path $HOME '.elomar\bin' }

    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('elomar-install-' + [guid]::NewGuid())
    New-Item -ItemType Directory -Path $tmp | Out-Null
    try {
        Write-Host "Downloading Elomar-Lang $version ..."
        Invoke-WebRequest "$base/$name.zip" -OutFile (Join-Path $tmp "$name.zip") -UseBasicParsing
        Invoke-WebRequest "$base/SHA256SUMS.txt" -OutFile (Join-Path $tmp 'SHA256SUMS.txt') -UseBasicParsing

        $line = Get-Content (Join-Path $tmp 'SHA256SUMS.txt') |
            Where-Object { ($_ -split '\s+')[-1].TrimStart('*') -eq "$name.zip" } |
            Select-Object -First 1
        if (-not $line) { throw "SHA256SUMS.txt has no line for $name.zip" }
        $expected = ($line -split '\s+')[0].ToLowerInvariant()
        # .NET directly: works in every PowerShell, whatever modules it can load.
        $stream = [IO.File]::OpenRead((Join-Path $tmp "$name.zip"))
        try {
            $sha = [Security.Cryptography.SHA256]::Create()
            $actual = (($sha.ComputeHash($stream) | ForEach-Object { $_.ToString('x2') }) -join '')
        } finally {
            $stream.Dispose()
        }
        if ($expected -ne $actual) { throw "checksum mismatch for $name.zip: the download is damaged" }

        Add-Type -AssemblyName System.IO.Compression.FileSystem
        [IO.Compression.ZipFile]::ExtractToDirectory((Join-Path $tmp "$name.zip"), $tmp)
        New-Item -ItemType Directory -Force -Path $dir | Out-Null
        $exe = Join-Path $dir 'elomar.exe'
        $old = "$exe.old"
        # A running elomar.exe cannot be replaced, but it can be renamed.
        if (Test-Path $old) { Remove-Item $old -Force -ErrorAction SilentlyContinue }
        if (Test-Path $exe) { Move-Item $exe $old -Force }
        Copy-Item (Join-Path $tmp "$name\elomar.exe") $exe
        if (Test-Path $old) { Remove-Item $old -Force -ErrorAction SilentlyContinue }
    } finally {
        Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }

    if ($env:ELOMAR_NO_PATH -ne '1') {
        $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
        $parts = @($userPath -split ';' | Where-Object { $_ })
        if ($parts -notcontains $dir) {
            [Environment]::SetEnvironmentVariable('Path', (@($dir) + $parts) -join ';', 'User')
            Write-Host "Added $dir to your PATH (open a new terminal to use it everywhere)."
        }
        if (@($env:Path -split ';') -notcontains $dir) { $env:Path = "$dir;$env:Path" }
    }

    & (Join-Path $dir 'elomar.exe') --version
    Write-Host "Elomar-Lang $version is installed: $(Join-Path $dir 'elomar.exe')"
    Write-Host 'Start with:  elomar new hello   (or: elomar new helpdesk --ai mock)'
}
