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
