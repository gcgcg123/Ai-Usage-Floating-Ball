# AI Usage Floating Ball

Windows 桌面 AI API 用量監測懸浮球。集中查看 Ollama、DeepSeek、OpenRouter、Kimi、xAI、Qwen、Groq、Mistral、Together AI 與自訂 API 的餘額、訂閱額度、重置時間和可用狀態。

API Key 儲存在 **Windows Credential Manager**，不會寫入設定檔、Snapshot、日誌或本 Repository。

## 功能

- 透明、Always-on-top 的 76×76 懸浮球。
- 滑鼠移入展開資訊卡，左右箭嘴切換供應商。
- 用量環、百分比／餘額標籤與官方供應商圖示。
- 可停靠螢幕左側、右側或上方，摺疊時只留 12px 提示邊。
- Ollama 可分別追蹤訂閱額度（Session／Weekly／Monthly）或加購餘額。
- DeepSeek 支援多幣種，可選 Auto／CNY／USD 作為球體主顯示幣種。
- 每間供應商獨立刷新排程，失敗不影響其他供應商。
- 資訊卡開啟時自動刷新超過 30 秒的資料；手動 ↻ 可強制查詢。
- 最後成功資料保留、Stale／Error 狀態和錯誤處理建議。
- Windows Tray：顯示／隱藏、Settings、結束。
- 開機自動啟動、系統通知與警示門檻。
- Settings 可新增、編輯、測試、排序、啟用／停用及刪除供應商。

## 介面

### 懸浮球

<p align="center">
  <img src="docs/screenshots/ball-ollama.png" width="160" alt="Ollama 懸浮球">
</p>

### Ollama 訂閱額度

<img src="docs/screenshots/info-ollama.png" width="420" alt="Ollama 訂閱額度資訊卡">

### DeepSeek 多幣種餘額

<img src="docs/screenshots/info-deepseek.png" width="420" alt="DeepSeek 多幣種餘額資訊卡">

### Settings

<img src="docs/screenshots/settings.png" width="720" alt="AI Usage Floating Ball Settings">

> README 截圖使用 `?demo=1` 固定示範資料，不包含真實 API Key 或帳戶餘額。

## 支援的供應商

| 供應商 | 接入方式 | 顯示內容 |
| --- | --- | --- |
| Ollama | 官方 API | 訂閱額度、加購餘額 |
| DeepSeek | 官方 API | CNY／USD 餘額 |
| OpenRouter | 官方 API | Key limit、credits、usage |
| Kimi 國際／中國 | 官方 API | 可用餘額、voucher、cash |
| Grok / xAI | Management API | Prepaid balance（需要 Team ID） |
| Qwen / DashScope | Custom endpoint | 依端點回傳的 quota／balance／usage |
| Groq | Custom endpoint | 依端點回傳的 quota／balance／usage |
| Mistral | Custom endpoint | 依端點回傳的 quota／balance／usage |
| Together AI | Custom endpoint | 依端點回傳的 quota／balance／usage |
| Custom API | JSON path mapping | 自訂餘額、上限、用量與重置時間 |

Qwen、Groq、Mistral、Together AI 的一般 Inference API Key 未必能查詢帳戶餘額；需要使用供應商提供的 Billing／Usage endpoint，否則請使用 Custom API。

## 安裝

1. 前往 [Releases](https://github.com/gcgcg123/Ai-Usage-Floating-Ball/releases/latest)。
2. 下載 `AI Usage Floating Ball_0.5.0_x64-setup.exe`。
3. Windows 10／11 需要 Microsoft Edge WebView2 Runtime（Windows 11 通常已內建）。
4. 安裝後從開始功能表啟動。

安裝檔目前未做程式碼簽名；SmartScreen 可能顯示「Windows 已保護您的電腦」。如果你信任此 Repository 的 Release 和 SHA-256，可選擇「仍要執行」。

## 第一次使用

1. 將滑鼠移到懸浮球，按齒輪開啟 Settings。
2. 在 Provider Catalog 選擇供應商。
3. 輸入 Display name 和 API Key。
4. 按 **Test Connection**。
5. 成功後按 **Save**。

API Key 在輸入後直接交給 Windows Credential Manager；再次編輯時留空即可保留原有 Key。

### Ollama 追蹤方式

- **自動（訂閱優先）**：有 Session／Weekly／Monthly 資料時使用訂閱額度，否則使用加購餘額。
- **只追蹤訂閱額度**：瓶子以剩餘百分比最低的訂閱窗口為準。
- **只追蹤加購餘額**：使用 Ollama purchased credit balance。

訂閱百分比門檻和加購餘額門檻是分開設定的，不會互相換算。

### DeepSeek 主要幣種

DeepSeek 可能同時回傳 CNY 和 USD。Settings 可選 Auto、CNY 或 USD：

- 球體使用主要幣種。
- 資訊卡仍會分開列出每個幣種的餘額、贈金與充值金額。

## 刷新策略

- 背景：每 5／10／15 分鐘依供應商設定刷新。
- 開啟資訊卡：資料超過 30 秒會自動刷新，15 秒內不重複查詢。
- 手動 ↻：一定重新查詢。
- 401／403、429、5xx、Timeout 等錯誤使用不同退避時間。
- 背景失敗時保留最後成功資料，並標記 Stale／Error。

供應商不會主動推送 balance webhook，因此「充值後即時」只能在 App 開啟資訊卡或按下刷新時重新查詢，無法保證其他地方充值後在同一秒反映。

## 安全與隱私

- API Key 只存於 Windows Credential Manager。
- `settings.json`、`snapshots.json` 不包含 API Key。
- 無帳號系統、無遙測、無分析追蹤。
- 只有當你加入並啟用供應商後，App 才向該供應商官方 API 發出請求。
- 所有 provider 查詢使用 HTTPS；除 localhost Custom API 外，HTTP endpoint 會被拒絕。
- API 查詢加入 `Cache-Control: no-cache`，減少舊餘額快取。
- Repository 提供 secret scan、`.gitignore`、Dependabot 和 CI 檢查。

詳細說明見 [SECURITY.md](SECURITY.md)。

## 本機資料

設定與最後成功 Snapshot 位於：

```text
%APPDATA%\com.personal.aiusage.liquidball
```

WebView2 快取位於：

```text
%LOCALAPPDATA%\com.personal.aiusage.liquidball
```

解除安裝不會自動刪除這些資料；需要徹底清除時可手動刪除上述資料夾。

## 開發

需求：

- Windows 10／11
- Node.js 22+
- Rust stable
- Microsoft C++ Build Tools 與 Windows SDK
- WebView2 Runtime

```powershell
npm install
npm run tauri dev
```

測試：

```powershell
npm test
npm run build
npm run check:secrets
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
```

Debug build（不產生 installer）：

```powershell
npm run tauri -- build --debug --no-bundle
```

Release installer：

```powershell
npm run tauri -- build
```

示範 UI：

```text
http://localhost:1420/?demo=1
http://localhost:1420/?demo=1&expanded=1
http://localhost:1420/?window=settings&demo=1
```

## 專案結構

```text
src/                  React UI、Settings、Demo data
src-tauri/src/        Tauri backend、Provider adapters、Scheduler
src-tauri/capabilities Windows capability permissions
docs/screenshots/     README screenshots
scripts/              Repository security checks
```

## 已知限制

- 不同供應商是否提供 balance／quota endpoint 取決於其公開 API。
- 一般 Inference Key 不一定有 Billing 權限。
- Tauri／WebView2 的透明視窗行為可能受 Windows GPU 和顯示設定影響。
- Installer 目前未簽名。
- 目前沒有自動更新器；更新時請下載新 Release 覆蓋安裝。

## Security

請不要在 Issue、Pull Request、截圖或測試資料中貼出 API Key、真實帳戶餘額或其他敏感資料。安全問題請使用 GitHub Security Advisory 私下回報。
