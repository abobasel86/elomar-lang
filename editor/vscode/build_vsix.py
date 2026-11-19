"""Packages this folder as a .vsix without Node tooling.

Usage:  python editor/vscode/build_vsix.py [output.vsix]
Then:   code --install-extension output.vsix

A .vsix is a zip with the extension under `extension/`, plus a manifest and
a content-types file.
"""

import json
import sys
import zipfile
from pathlib import Path
from xml.sax.saxutils import escape

HERE = Path(__file__).resolve().parent
FILES = [
    "package.json",
    "extension.js",
    "language-configuration.json",
    "README.md",
    "syntaxes/elomar.tmLanguage.json",
    "snippets/elomar.json",
]


def main() -> None:
    pkg = json.loads((HERE / "package.json").read_text(encoding="utf-8"))
    name, publisher, version = pkg["name"], pkg["publisher"], pkg["version"]
    default_out = HERE.parents[1] / "target" / f"{publisher}.{name}-{version}.vsix"
    out = Path(sys.argv[1]) if len(sys.argv) > 1 else default_out
    out.parent.mkdir(parents=True, exist_ok=True)

    manifest = f"""<?xml version="1.0" encoding="utf-8"?>
<PackageManifest Version="2.0.0" xmlns="http://schemas.microsoft.com/developer/vsx-schema/2011">
  <Metadata>
    <Identity Language="en-US" Id="{escape(name)}" Version="{escape(version)}" Publisher="{escape(publisher)}" />
    <DisplayName>{escape(pkg["displayName"])}</DisplayName>
    <Description xml:space="preserve">{escape(pkg["description"])}</Description>
    <Categories>{escape(",".join(pkg["categories"]))}</Categories>
    <Properties>
      <Property Id="Microsoft.VisualStudio.Code.Engine" Value="{escape(pkg["engines"]["vscode"])}" />
    </Properties>
  </Metadata>
  <Installation>
    <InstallationTarget Id="Microsoft.VisualStudio.Code" />
  </Installation>
  <Dependencies />
  <Assets>
    <Asset Type="Microsoft.VisualStudio.Code.Manifest" Path="extension/package.json" Addressable="true" />
    <Asset Type="Microsoft.VisualStudio.Services.Content.Details" Path="extension/README.md" Addressable="true" />
  </Assets>
</PackageManifest>
"""
    content_types = """<?xml version="1.0" encoding="utf-8"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
  <Default Extension=".json" ContentType="application/json" />
  <Default Extension=".js" ContentType="application/javascript" />
  <Default Extension=".md" ContentType="text/markdown" />
  <Default Extension=".vsixmanifest" ContentType="text/xml" />
</Types>
"""
    with zipfile.ZipFile(out, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("[Content_Types].xml", content_types)
        z.writestr("extension.vsixmanifest", manifest)
        for rel in FILES:
            z.write(HERE / rel, f"extension/{rel}")
    print(out)


if __name__ == "__main__":
    main()
