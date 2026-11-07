# Release Checklist

## Before building

- [ ] Bump `package.json`, `src-tauri/Cargo.toml`, and `src-tauri/tauri.conf.json`.
- [ ] Run `npm test`.
- [ ] Run `npm run build`.
- [ ] Run `npm run check:secrets`.
- [ ] Run Cargo tests and Clippy with `-D warnings`.
- [ ] Verify no real API keys or account balances appear in screenshots.
- [ ] Test Ollama subscription and balance modes.
- [ ] Test DeepSeek Auto/CNY/USD modes and refresh behavior.

## Build

```powershell
npm run tauri -- build
```

Artifacts:

```text
src-tauri/target/release/bundle/nsis/
```

## Publish

- [ ] Calculate SHA-256 for the installer.
- [ ] Create a Git tag `v0.5.0`.
- [ ] Create a GitHub Release.
- [ ] Upload the NSIS installer as a Release asset.
- [ ] Add SHA-256 and known limitations to Release notes.
- [ ] Verify README and Releases links.
