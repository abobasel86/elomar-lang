# Releasing Elomar-Lang

## A new version

1. Move the `## [Unreleased]` notes in `CHANGELOG.md` under a new heading
   `## [X.Y.Z] - YYYY-MM-DD`.
2. Set `version = "X.Y.Z"` in `Cargo.toml` (workspace) and in
   `editor/vscode/package.json`, and add the version to
   `editor/vscode/CHANGELOG.md`.
3. Commit, then push a tag:

   ```text
   git tag -a vX.Y.Z -m "Elomar-Lang X.Y.Z"
   git push origin vX.Y.Z
   ```

The **Release** workflow builds `elomar` for Windows, Linux and macOS and
the `.vsix`, and creates the GitHub release with the changelog notes. The
**VS Code extension** workflow publishes the extension to the stores that
have a token (below).

## One-time setup for the extension stores

Both stores are free. The tokens are secrets of the repository; they are
never written in a file.

### VS Code Marketplace

1. Sign in at <https://marketplace.visualstudio.com/manage> with a Microsoft
   account and create a publisher with the ID **`abobasel86`** (it must
   match `publisher` in `editor/vscode/package.json`).
2. At <https://dev.azure.com>, open **User settings → Personal access
   tokens**, and create a token with **Organization: All accessible
   organizations** and the scope **Marketplace → Manage**.
3. In the GitHub repository, open **Settings → Secrets and variables →
   Actions → New repository secret**, name it `VSCE_PAT`, and paste the token.

### Open VSX (VSCodium, Cursor and other editors)

1. Sign in at <https://open-vsx.org> with GitHub and accept the publisher
   agreement in your profile.
2. Create a namespace named `abobasel86` (`npx ovsx create-namespace
   abobasel86 -p <token>`) and an access token in **Settings → Access
   Tokens**.
3. Add it as the repository secret `OVSX_PAT`.

After that, the next version tag publishes the extension automatically. To
publish the current version without a new tag, run the **VS Code
extension** workflow by hand on that tag.

---

## بالعربية: إعداد متاجر الإضافات (مرة واحدة)

1. **متجر VS Code:** سجّل الدخول في <https://marketplace.visualstudio.com/manage> وأنشئ
   ناشرًا (publisher) معرّفه **`abobasel86`**. ثم من <https://dev.azure.com> افتح
   **User settings ← Personal access tokens** وأنشئ رمزًا لكل المؤسسات بصلاحية
   **Marketplace → Manage**. وأضفه في GitHub من **Settings ← Secrets and variables ←
   Actions** باسم `VSCE_PAT`.
2. **Open VSX** (لمحررات VSCodium وCursor وغيرها): سجّل الدخول في <https://open-vsx.org>
   بحساب GitHub، ووافق على اتفاقية الناشر، وأنشئ مساحة اسمها `abobasel86` ورمز وصول،
   وأضفه باسم `OVSX_PAT`.

بعدها يُنشر كل إصدار جديد من الإضافة تلقائيًا عند دفع وسم الإصدار. الرموز سرية ولا تُكتب
في أي ملف.
