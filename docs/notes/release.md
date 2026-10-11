| `.github/workflows/ci.yml` | push main、PR | `core`（Linux）fmt / clippy / 全 workspace 測試（排除 IMK 殼）；`macos` 編 IMK 殼並跑它的測試；`windows` 編 Server / TSF DLL / Settings 並跑測試。倉庫公開，Actions 不計費 |
| `.github/workflows/audit.yml` | 每週一、Cargo.lock 變動 | `cargo audit`（RustSec 已知漏洞） |
| `.github/dependabot.yml` | 每週一 | Cargo 依賴與釘 commit 的 actions 的更新 PR |# 發版流程

2026-09-07 搭起來的：GitHub Actions 按標籤打包、建 Release、生成官網下載頁用的 `releases.json`。
這裡記怎麼發一版、各環節的依賴，以及官網怎麼消費產物。

## 一次發版做什麼

0.1.4 起三個平臺共用版本號、共用一個 GitHub Release（標籤 `v<版本>`）：同一個版本號、同一份更新日誌只出現一次，三個平臺的安裝包都掛在上面。

1. 改版本號，把 `-dev` 去掉：`apps/macos/Cargo.toml`（Info.plist 與 pkg 檔名從這裡取）、`apps/windows/{server,tsf,settings}/Cargo.toml`（三個一起改，打包讀 `server`）、
   `apps/linux/server/Cargo.toml`，然後 `cargo update --workspace --offline` 同步 `Cargo.lock`。
   **發版之間版本號一直帶 `-dev`**（Rust nightly / Firefox Nightly 那套）：Cargo.toml 寫 `0.1.5-dev`，打包指令碼再接上 git 短雜湊，
   本地裝的、CI 中間構建的都顯示 `0.1.5-dev-1a2b3c4`（工作區有改動加 `+`），測試時一眼知道裝的是哪個提交；版本號乾淨的一定是線上包；帶 `-dev` 的標籤 CI 直接拒絕。
   pkg 的 `--version`、Inno 的 `VersionInfoVersion` 只認數字點號，`bundle.sh` / `build.ps1` 去掉字尾再傳，檔名保留完整版本。
2. `CHANGELOG.md` 頂上那一節把「未釋出」換成日期：`## <版本> · <日期> · <渠道>`，一行一條、面向使用者的措辭；只屬於一個平臺的條目加「macOS：」「Windows：」「Linux：」字首。
   渠道是**更新渠道**（2026-09-17 定）：定期發的版本標 `stable`、版本號乾淨（`0.1.4`）；中間放給測試者的版本標 `alpha` / `beta` / `rc`，
   版本號帶同名預釋出字尾（`0.1.5-beta.1`），`release.yml` 見到字尾就把 GitHub Release 標成預釋出、不搶 latest。
   「產品還在測試期」由 0.x 的版本號表達，不佔渠道欄位；0.1.2 及更早的條目標的 `beta` 是舊含義，不回改。
   **更新日誌手寫，不由提交自動生成**：發版前按上個標籤以來的 `git log` 起草幾條，人審一遍再定稿。
   標題下可以寫一行 `網盤：[夸克網盤](連結)`（GitHub 下載不方便的使用者用）：它不算更新條目，Release 說明把它放在最前面，`releases.json` 單獨輸出成 `mirrors`，官網顯示成按鈕。
   網盤檔案在 CI 跑完後手動上傳。
3. 提交，打註釋標籤並推：`git tag -a v0.1.4 -m "青簡 0.1.4" && git push origin main v0.1.4`。
4. 標籤推出去之後緊接一個普通提交把版本號改成下一個開發版（`0.1.5-dev`），CHANGELOG 頂上加 `## 0.1.5 · 未釋出 · stable`；-dev 版本永遠沒有標籤與 Release。

`release.yml` 的流程：

- `prepare`（`ubuntu-26.04`）：門禁（要打的平臺版本號都與標籤一致、不帶 `-dev`、標籤在 main 上、配了索引簽名金鑰）→ 用 CHANGELOG 那一節加各平臺未簽名提示建**草稿** Release。
- `macos`（`macos-26`）、`windows`（`windows-latest`）、`linux`（`ubuntu-26.04`）並行：下載產品資料 → 打包 → `tools/release/upload-build.sh` 把安裝包和
  這個平臺的 `SHA256SUMS-<平臺>`、`build-info-<平臺>.json` 片段傳到草稿上。
- `publish`：三個平臺都成功才跑，`tools/release/finish-release.sh` 把片段合成一份 `SHA256SUMS` 與 `build-info.json`（各平臺構建機記在 `platforms` 裡）、刪掉片段、草稿轉正
  → `publish-releases-json.sh` 生成並簽名 `releases.json` → `bump-website.sh` 觸發官網構建。
  任何平臺失敗，Release 就停在草稿、使用者看不到；修好後刪掉草稿與標籤重推，或在 Actions 頁重跑失敗的 job。

**單平臺補丁**（只修一個平臺的急事）：只改那個平臺的版本號，打 `macos-v<版本>` / `windows-v<版本>` / `linux-v<版本>`，同一套流程只打那一個平臺，Release 標題帶平臺名。

發版後 GitHub Release 上有：`qingjian-<版本>-macos-arm64.pkg`、`-macos-x86_64.pkg`、`-windows-x86_64-setup.exe`、`-linux-x86_64.tar.gz`、`SHA256SUMS`、`build-info.json`、`releases.json` 與 `.sig`。

官網由 Cloudflare Workers Builds 按官網倉庫的提交自動構建，沒有可呼叫的構建鉤子，所以主倉庫靠**往官網倉庫推一個小提交**來觸發：
`tools/release/bump-website.sh` 把版本標籤與文件提交號寫進官網的 `src/content/upstream.json` 並提交推送（提交者 qingjian-ci）。
配了 `QINGJIAN_WEB_TOKEN`（對 qingjian-web 有 Contents: read and write 的 fine-grained PAT）release.yml 末尾自動做；
官網文件只隨發版更新（`docs/user/` 平時改動不推官網，免得文件領先於使用者裝到的版本）。沒配就在官網倉庫隨便提交一次（或本地跑這個指令碼）。
官網構建時才拉最新 Release 的 `releases.json` 與主倉庫 `docs/user`，所以提交內容本身不重要，`upstream.json` 只是留個記錄、順便讓文件按記下的提交號拉（版本對得上）。

Rust 工具鏈由 `rust-toolchain.toml` 釘版本（現在 1.96.0），兩個 workflow 裡 `dtolnay/rust-toolchain` 的 `toolchain:` 輸入寫同一個號；升級 Rust 時三處一起改。

## 各平臺的特別之處

- **Windows**：沒有程式碼簽名證書時 workflow 設 `QINGJIAN_UIACCESS=0`：沒簽名的 exe 帶 uiAccess=true 起不來。
  代價是候選窗在工作列搜尋 / 設定這類 UWP 宿主裡可能被蓋住，使用者文件與 CHANGELOG 已列為已知問題。
  Certum 開源證書辦下來後：在 `build.ps1` 加 signtool 一步（`sign-local.ps1` 是本機自籤的參考），workflow 去掉那個環境變數；SmartScreen 對無簽名安裝包的攔截也一併消失。
  Inno Setup 釘 7.1.0（與開發機同版本、釘死 GitHub Release 上的安裝程式與雜湊）。
- **Linux**：`apps/linux/scripts/package.sh` 打的包裡是編好的 Server 與 Fcitx5 外掛、產品資料壓縮包、`install.sh` / `uninstall.sh` / `files.py`，按倉庫相對路徑放，
  安裝與原始碼安裝走同一份清單與校驗；打完先裝進臨時目錄冒煙測試（檔案齊、校驗過、`ldd` 無缺庫）再上傳。
  外掛連結構建機的 Fcitx5（`build-info.json` 的 `platforms.linux.runner` 記了版本），所以只承諾 Ubuntu 26.04，其他發行版走原始碼安裝。
- **官網**：安裝包按檔名識別平臺（`ASSET_KINDS`），下載頁按訪問者平臺取「有該平臺安裝包的最新版本」（`latestFor`）；0.1.3 及更早按平臺分開的 Release 照樣識別，同版本號合在一條裡。

## 提交前檢查與 CI

本地 `git config core.hooksPath .githooks` 啟用一次後，每次提交前 `.githooks/pre-commit` 先拒絕裝飾性分隔註釋（`// ====` / `// ────`，只做視覺分組不帶「為什麼」），再跑 `cargo fmt --check` 與 `cargo clippy -D warnings`（含 IMK 外殼，增量幾十秒）；
`.githooks/pre-push` 在推之前跑全 workspace 測試。外部 PR 走同一套 `ci.yml`，不過不合。

供應鏈：workflow 裡的 actions 一律釘到 commit（註釋寫對應標籤），`.github/dependabot.yml` 每週一提 Cargo 與 actions 的更新 PR；`audit.yml` 每週與 Cargo.lock 變動時跑 `cargo audit`；
cargo 命令全 `--locked`（含 `bundle.sh` 與 `build.ps1`）。普通 CI 只有 `contents: read`，checkout 不留憑據；release 的 secrets 不放頂層 env，只注入用它的那一步。

發版門禁（`release.yml` 第一步）：版本號與標籤一致且不帶 `-dev`；標籤指向的提交必須在 `main` 上（`git merge-base --is-ancestor`）；產品資料下載後按 `data` Release 的 `SHA256SUMS` 校驗，摘要寫進 `build-info.json` 的 `data_sha256`。
**正式版前還欠**：產品資料改成不可變 tag 並在倉庫裡鎖定版本（現在滾動覆蓋，同一原始碼 tag 重跑可能拿到不同資料）、安裝包內容驗證（詞庫 / 模型 / 許可齊不齊、簽名校驗）。

## 兩個 workflow

| 檔案 | 觸發 | 做什麼 |
|---|---|---|
| `.github/workflows/ci.yml` | push main、PR | Linux 上 `cargo fmt --check` / clippy / test，排除 `qingjian-macos`（IMK 外殼只能在 macOS 編譯，macOS runner 計費是 Linux 的 10 倍） |
| `.github/workflows/release.yml` | 推 `v*`（三個平臺）或 `macos-v*` / `windows-v*` / `linux-v*`（單平臺）標籤 | `prepare` 門禁並建草稿 Release → `macos` / `windows` / `linux` 並行打包上傳 → `publish` 合併摘要、轉正、生成 `releases.json`、觸發官網構建（見上文） |

## 產品資料從哪來

詞庫、語言模型、釋義表（`data/generated/*.qj`、`dicts/*.qj`、英文詞表）不在 git 裡，體積約 90 MB 且由本機資料管道生成。
它們發在倉庫裡一個個**不可變**的預釋出 Release 上：`data-v1`、`data-v2`……每次資料重生成發一個新號、從不覆蓋
（預釋出不會成為 GitHub 的 latest，官網取 latest 時不會拿到它）。倉庫裡 `tools/release/data.lock` 釘住當前要用的標籤與資產的 SHA-256，
跟用到新資料的程式碼同一個提交進去：checkout 哪個提交就拿到它對應的那版資料，離線自編譯的人不會因為我們改了資料而編出壞包。

- `tools/release/data-bundle.sh`：把 `data/generated/`、含章·知微 `hanzhang-zhiwei-small.qjm` 與含章·通變 `hanzhang-tongbian-small.qjm` 裝進唯一附件 `qingjian-data.tar.gz`
  （匯出的三件套放各自模型目錄，`tools/release/pack-model.sh` 打成 `.qjm` 容器）。訓練中間產物不進入公開包；發到下一個 `data-vN`（`--tag` 可指定，已存在就拒絕），然後改寫 `data.lock`。
- `tools/release/data-fetch.sh`：按 `data.lock` 下載（有 gh 用 gh，沒有就 curl 直連）、按鎖檔案裡的雜湊校驗，
  資料包解到 `data/generated/` 與兩份模型各自的 `data/models/<標識>/`。舊鎖仍可讀取分離附件。`release.yml` 三個平臺的 job 和離線自編譯走同一個指令碼；
  `bundle.sh`、`qingjian.iss` 與 Linux 打包指令碼將兩份模型分別裝入對應目錄，產品端優先使用通變。
  標籤與各資產雜湊記進 `build-info.json`（`data_tag` / `data_sha256` / `model_sha256` / `p2c_model_sha256`）。

資料重生成之後（重跑 lexicon / bigram / gloss-gen export）或模型重訓之後跑一次 `data-bundle.sh`（三件套比 `.qjm` 新會自動重打），
把鎖檔案的改動提交（`chore(data): 資料 data-vN`），否則 CI 打的包還是鎖檔案指的舊資料。模型檔案缺失或雜湊不符時 CI 會失敗，不會靜默地發出錯資料的包。
2026-09-16 之前用的是滾動覆蓋的 `data` Release，已凍結不再更新。

## 版本索引的簽名

軟體內「檢查更新」只認簽過名的 `releases.json`（設計見 `docs/design/update.md`）。`publish-releases-json.sh` 生成索引後用 `tools/release-sign` 簽出 `releases.json.sig`，
兩個檔案一起掛到本次 Release 與 GitHub latest；官網構建時原樣拷到 `https://qingjian.app/releases.json` 與 `.sig`。

- 私鑰是倉庫 Secret `QINGJIAN_INDEX_SIGNING_KEY`（base64 的 32 位元組）；沒配時 `release.yml` 的門禁直接失敗。維護者本機留一份在 `~/.config/qingjian/index-signing.key`（不進倉庫）。
- 換鑰：`cargo run -p qingjian-release-sign -- keygen --out <新檔案>`，把列印的公鑰加進 `crates/qingjian-update/src/index/signature.rs` 的 `PUBLIC_KEYS`（新舊並列），
  發一兩個版本後再換 Secret、去掉舊公鑰。直接換 Secret 會讓所有已裝版本收不到更新。
- 改了 CHANGELOG 後用 `publish-releases-json.sh` 重刷索引同樣要帶這個環境變數。

## 簽名與公證

沒有證書時 CI 照樣出包（ad-hoc 簽名，Release 說明裡自動加一句「首次開啟要在隱私與安全性裡放行」）。
Apple Developer 賬號有了以後，在倉庫 Secrets 裡配齊 `release.yml` 頭部註釋列的七個值（.p12 與 .p8 都 base64），
下一次發版就是簽名 + 公證 + 釘票據的包，使用者下載雙擊即裝。`bundle.sh` 本身通過 `QINGJIAN_SIGN_IDENTITY` /
`QINGJIAN_INSTALLER_IDENTITY` / `QINGJIAN_NOTARY_PROFILE` 三個環境變數工作，本機有證書也能這樣打。

## releases.json：官網下載頁的資料來源

`tools/release/releases_json.py` 從 `CHANGELOG.md`（日期、渠道、更新日誌）、GitHub Releases API（附件、地址、大小）
與每次釋出的 `SHA256SUMS` / `build-info.json`（每個包的 sha256、提交雜湊、構建時間、工具鏈）生成，掛在每個版本的 Release 上；官網固定取
`https://github.com/<repo>/releases/latest/download/releases.json`（倉庫私有期間要帶令牌走 API 下載附件）。

結構對應官網 `src/lib/releases.ts` 裡的 `Release` / `Asset` 型別：

```json
{
  "schema_version": 1,
  "generated": "2026-09-07T12:00:00Z",
  "repository": "owner/qingjian",
  "latest": "0.1.3",
  "releases": [
    {
      "version": "0.1.3",
      "date": "2026-09-18",
      "channel": "stable",
      "notes": ["整句輸入：……", "候選旁有詞性和譯詞……"],
      "mirrors": [{ "name": "夸克網盤", "url": "https://pan.quark.cn/s/…" }],
      "commit": "869ad00…（40 位）",
      "built_at": "2026-09-07T08:38:12Z",
      "toolchain": "rustc 1.96.0 (ac68faa20 2026-05-25)",
      "assets": [
        { "platform": "macos", "arch": "Apple Silicon", "cpu": "arm64", "file": "qingjian-0.1.3-macos-arm64.pkg",
          "url": "https://github.com/owner/qingjian/releases/download/macos-v0.1.3/qingjian-0.1.3-macos-arm64.pkg",
          "size": 35989277, "sha256": "…" },
        { "platform": "macos", "arch": "Intel", "cpu": "x86_64", "file": "qingjian-0.1.3-macos-x86_64.pkg", "url": "…", "size": 36172871, "sha256": "…" }
      ]
    }
  ]
}
```

- `releases` 從新到舊，`latest` 是第一條的版本號；官網「當前版本」取它，歷史版本列表就是整個陣列。
- `channel` 是 `alpha` / `beta` / `rc` / `stable`，顯示成什麼字由官網定；`commit` / `built_at` / `sha256` 給使用者核對下載的包，下載頁應顯示 sha256 與提交短雜湊。
- 安裝包檔名固定為 `qingjian-<版本>-<平臺>-<cpu>[-setup].<副檔名>`（全小寫；平臺 `macos` / `windows` / `linux`，cpu `arm64` / `x86_64`，2026-09-17 定，
  包管理器的地址模板與檢查更新都靠它穩定）。平臺與架構由檔名判定（`ASSET_KINDS`，同時認 0.1.2 及更早的 `Qingjian-<版本>-arm64.pkg` / `-Setup.exe` 舊名），
  `arch` 給人看（Apple Silicon / Intel / x64），`cpu` 給程式比對。
- `schema_version` 現在是 1：只加欄位不用動，改了已有欄位的含義或結構才加一。
- `mirrors` 來自 CHANGELOG 那一節的「網盤：」行，沒有就是空陣列；官網在下載按鈕旁單獨顯示。
- `SHA256SUMS` 與 `releases.json` 自己不列進 `assets`。
- 官網側要做的：構建時下載這個檔案替代手寫的 `releases` 陣列（與拉 `docs/user` 的 `sync-docs.mjs` 同一處、同一個令牌），
  `downloadsOpen` 開關仍由官網自己控制。

## 本機打包

`apps/macos/scripts/bundle.sh --pkg` 打本機架構；`QINGJIAN_TARGET=x86_64-apple-darwin` 交叉編譯 Intel 包（要先 `rustup target add`，
本機不需要時不必裝，CI 上兩個都打）。成品在 `target/pkg/qingjian-<版本>-macos-<arch>.pkg`，每個架構一個工作目錄，連著打互不覆蓋。
