# Security Policy

## Supported versions

Security fixes are provided for the latest published release.

## Reporting a vulnerability

Please use the Repository's **Security → Advisories → Report a vulnerability** flow. Do not open a public Issue containing:

- API keys or access tokens
- real account balances
- personal data
- private endpoint URLs
- screenshots that expose provider accounts

If a key may have leaked, revoke it at the provider immediately and replace it in Settings.

## Secret handling

- API Keys are stored in Windows Credential Manager.
- `settings.json` and `snapshots.json` do not contain secrets.
- API Keys are never intentionally logged or sent to telemetry.
- Repository CI runs `npm run check:secrets`.
- Screenshots in this Repository use fixed demo data.

## Network access

The app sends requests only to providers configured by the user and only while those profiles are enabled. Native provider endpoints use HTTPS. Custom endpoints must also use HTTPS, except localhost testing endpoints.

## Local data

Configuration and last-known-good snapshots are stored under:

```text
%APPDATA%\com.personal.aiusage.liquidball
```

WebView2 profile data is stored under:

```text
%LOCALAPPDATA%\com.personal.aiusage.liquidball
```

Uninstall may leave these directories in place. Remove them manually if full data deletion is required.

## Release integrity

Each Release includes a SHA-256 hash. Installers are currently unsigned; verify the hash and Release source before running them.

### Unsigned releases / 未簽名版本

An unsigned installer has no trusted Authenticode code-signing signature. Windows therefore cannot verify the publisher identity or detect post-packaging modification through the signature. SmartScreen may show an unknown-publisher warning; this is not proof that the file is malicious.

未簽名安裝檔沒有受信任的 Authenticode 數碼簽名，Windows 無法透過簽名確認發佈者身份或檔案是否在打包後被修改。SmartScreen 可能顯示未知發佈者警告，但這不代表檔案一定是惡意程式。

Always verify the published SHA-256 before running an unsigned installer:

```powershell
Get-FileHash -Algorithm SHA256 `
  -LiteralPath ".\AI.Usage.Floating.Ball_0.5.0_x64-setup.exe"
```

A valid code signature proves publisher identity and file integrity; it does not prove that software contains no vulnerabilities. Signing options include an OV/EV code-signing certificate, Azure Trusted Signing, or a trusted signing service.

有效的數碼簽名可證明發佈者身份與檔案完整性，但不代表程式沒有漏洞。常見方案包括 OV／EV Code-signing Certificate、Azure Trusted Signing 或受信任的簽名服務。
