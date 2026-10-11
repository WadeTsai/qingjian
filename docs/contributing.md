# 開發約定

改程式碼前先看這一頁：架構上不能越的線、程式碼怎麼組織、版本號與提交資訊怎麼寫、改了行為要同步哪些文件。
設計的來龍去脈在 [design/architecture.md](design/architecture.md)，各 crate 的實現要點在 [notes/crate-notes.md](notes/crate-notes.md)。

## 架構約束（不要違反）

長版與理由見 [design/architecture.md](design/architecture.md)。

- **Core 與平臺層嚴格解耦。** `qingjian-core` 及其兄弟 crate 必須平臺無關：詞庫、拼音解析、候選生成、排序、學習、翻譯、文本變換全部屬於 Core。
  平臺層（IMK / TSF / IBus-Fcitx）只做兩件事：把系統輸入事件翻譯成 Core 的輸入，把 Core 返回的幀畫到候選視窗。
  **平臺層裡不允許出現排序邏輯、詞庫訪問、翻譯呼叫或文本變換。** 判斷標準：把 IMK 換成 TSF，不應該需要改 Core 的任何一行。
- **一個候選詞只顯示一種輔助語言。** 使用者配置 Primary Language + 單個 Learning Language。不要設計成 `translations: Vec<Translation>` 或
  `HashMap<Lang, String>` 這類多語言並列的資料結構，那會在 API 層面把「一次只學一種語言」這條產品原則給破壞掉。翻譯是候選詞的 annotation（可選、單條）。
- **輸入優先於學習。** 任何為學習功能增加的延遲、彈窗、UI 干擾都是設計錯誤。翻譯查詢不能阻塞候選生成，Core 必須能在翻譯尚未就緒時先返回候選。
- **輸入方案是配置項，不是模式。** 雙拼、注音這類鍵盤方案放 `[general]` 裡當設定，中 / 英切換始終是布林；新方案不能改變別的方案的既定按鍵行為（[user/getting-started/keys.md](user/getting-started/keys.md)）。
- **顯示面自繪、控制元件面原生。** 候選窗、拼音行、狀態條這類顯示面由渲染器出點陣圖各平臺貼圖（主題靠它）；偏好設定、選單、安裝器用各平臺原生控制元件。見 [design/rendering.md](design/rendering.md)。

## 程式碼組織

- **一個型別一個檔案。** 一個 struct / enum / trait 及其 impl 單獨一個檔案；模組檔案只做 `mod` 宣告、re-export 與自由函式，不把一個 crate 平鋪在 `lib.rs` 裡。
- **子模組用目錄。** 有子模組的模組用 `foo/mod.rs`，**不用** `foo.rs` + `foo/` 並列。
- **同詞幹的兄弟檔案收進目錄，絕不用檔名字首分組。** `key_event.rs` + `key_outcome.rs` → `key/mod.rs` + `key/{event,outcome}.rs`，哪怕沒有 `key.rs` 這個共同父檔案；
  `query/` + `querying.rs` 這種也不行。判斷：兩個及以上檔名共享一段字首且同屬一個概念，就收進以那段字首命名的目錄。
- **一個職責連帶它專用的型別收進一個目錄。** `engine/commit/mod.rs` 放上屏部分，`chain.rs` / `last.rs` 放只有它用的型別。
- **大型別的 `impl` 按職責拆成子模組。** 每個檔案一個 `impl Foo { … }`（`host/settings.rs` 這樣），結構體與構造留在 `mod.rs`，跨檔案用到的私有方法標 `pub(super)`。
- **檔案長度。** 單檔案不超過 800 行，目標 500 行以內；測試超過 200 行搬到 `tests.rs`（多時 `tests/` 按主題分檔案）。
- **匯入寫精確路徑。** 不用 `use super::*` / `use foo::*`（`#[cfg(test)] mod tests` 裡的 `use super::*` 除外）。
  子檔案從父模組拿的型別寫 `use super::{Engine, Candidate};`，父模組沒定義的直接 `use crate::…` 或 `use <crate>::…`，不繞 `super::` 轉手。
  存量的 glob 改到那個檔案時順手換掉，不專門清掃。

## 命名與註釋

- 程式碼識別符號一律英文，註釋與文件用中文；`thiserror` 的 `#[error]` 文案用英文，日誌與 UI 文案用中文。
- 新檔案都要有 `//!` 檔案頭；結構體 / 列舉欄位之間空一行，欄位名說不清的加 `///`（`r` / `g` / `b`、`width` 這類不用，`a`「255 為不透明」這類要）。
- 註釋只寫維護時用得上的：檔案頭一兩句說它是什麼、怎麼用；程式碼裡只解釋讀程式碼看不出來的約束與原因（平臺怪癖、協議約定、踩過的坑），一兩句說完。
  不寫複述程式碼的教學性註釋，不寫改動經過、除錯過程這類日誌型註釋；較長的設計理由寫進 `docs/`，註釋裡留一句指過去。
- 不寫裝飾性分隔註釋（`// ====`），提交鉤子會攔。

## 依賴與配置

- 依賴用 `cargo add` 加，共用包提到根 `[workspace.dependencies]`；錯誤用 `thiserror` 不用 `anyhow`；日誌用 `tracing` 門面。
- 快捷鍵一律進 `[shortcut]` 可配置，不寫死鍵碼。

## 版本號

- `crates/*` 用 `version.workspace = true`；**`apps/*` 各殼是獨立釋出的產品，寫死自己的 `version`**（Windows 讀 `server/Cargo.toml`）。
- 發版之間帶 `-dev`（兩端都是 `0.1.3-dev`），打包指令碼再接 git 短雜湊成 `0.1.3-dev-1a2b3c4`（髒加 `+`，Cargo.toml 裡只寫 `-dev`）。
- 發版提交去掉 `-dev` 打標籤 `v<版本>`（三個平臺共用一個 Release；單平臺補丁用 `macos-v` / `windows-v` / `linux-v<版本>`），標籤後再改成下一個 `-dev`；帶 `-dev` 的標籤 CI 拒絕；pkg / Inno 只認數字點號。

## 提交資訊

- [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)：第一行 `<型別>(<範圍>): <說明>`，型別與範圍英文小寫，說明用中文，例如
  `fix(core): 修自繪輸入框吞數字`、`feat(windows): 三程序日誌統一到 %LOCALAPPDATA%\Qingjian\logs`、`docs(changelog): 補 0.1.3 條目`。
  - 型別：`feat` 新功能 / `fix` 修 bug / `docs` 只改文件 / `refactor` 不改行為的整理 / `perf` 效能 / `test` 只改測試 /
    `build` 打包與構建指令碼 / `ci` 工作流 / `chore` 版本號、依賴、倉庫雜務 / `style` 只改格式 / `revert` 還原。
  - 範圍：crate 或殼的名字——`core` `platform` `render` `dictionary` `translate` `learning` `predict` `lm` `neural` `format` `cli`
    `macos` `windows`（Server / DLL / 設定程式細分時用 `server` `tsf` `settings`）`installer` `linux` `tools` `docs` `ci` `deps` `release`；
    跨好幾處的可以省略。不相容的改動在範圍後加 `!`。
  - 正文寫「為什麼」與取捨，一行一條。`.githooks/commit-msg` 會攔第一行不合格式的提交，合併提交也一樣（`git merge --no-ff -m "feat: …"`）。
  - 2026-09-16 之前的歷史是「`macOS：……` / `Core：……`」的中文冒號格式，不重寫。

## 文件同步

- 實現與規劃分歧時以程式碼為準並改文件。技術方向寫 `docs/design/`，計劃寫 `docs/plan/`，工程記錄寫 `docs/notes/`；不往 README 里加技術內容。
- **使用者能感知的行為改了（按鍵、選單、偏好設定、配置檔案、資料檔案），同一個提交裡改 `docs/user/` 對應的頁**，按鍵改動同時改 `keys.md`。
- 改了某個 crate 的實現要點（資料檔案、常數、生成命令），同步 `docs/notes/crate-notes.md`。

## 提交前檢查

- 鉤子：`.githooks/pre-commit`（禁裝飾性分隔註釋 + fmt + clippy）、`.githooks/commit-msg`（提交資訊格式）、`.githooks/pre-push`（全 workspace 測試）；`git config core.hooksPath .githooks` 啟用一次。
  兩個跑編譯的鉤子按 `uname` 劃平臺範圍：**非 Apple 平臺排除 `qingjian-macos`**（IMK 殼依賴 objc2，在別的平臺上是硬 `compile_error!`，
  排除不掉就整條命令失敗），與 [ci.yml](../.github/workflows/ci.yml) 三個 job 的劃分一致；Server 的 uiAccess 預設不帶
  （沒簽名的 exe 帶 uiAccess=true 起不來，os error 740），只有 `build.ps1 -Sign` 設 `QINGJIAN_UIACCESS=1` 開啟。
- 排序 / 整句 / 糾錯的改動先跑 `apps/cli` 再合。

## CI 與發版

- CI 三個 job（Linux 全量 / macOS 殼 / Windows 三 crate）都 `--locked`；Dependabot 升 actions；每週 `cargo audit`。
- 發版：推 `<平臺>-v<版本>` 標籤觸發 `release.yml`，門禁是版本號 = 標籤且不帶 -dev、標籤在 main 上、產品資料按 SHA256SUMS 校驗。
- CHANGELOG 手寫、發版時由維護者統一改（PR 不動它）。流程與 Secrets 見 [notes/release.md](notes/release.md)。

## 外部 PR

- 從 main 開分支，一個 PR 只做一件事、只碰一個平臺（Core 改動單獨一個）。
- 維護者對著 main 審，squash 合併保留作者署名。PR 模板裡的合併前清單就是稽核標準。
- **修 bug、改殼裡行為（按鍵、上屏、候選窗位置）的 PR 必須真機驗過**：先在自己機器上覆現問題，改完在同一個應用裡確認修好，
  PR 的「怎麼驗證的」寫明系統版本、應用與操作步驟。編譯與 CI 通過不算驗證。沒驗過的修復發出去，報 issue 的人升級後還得再報一次。
  復現不了的（沒有那個應用或系統）不提修復：把分析寫在 issue 裡，或者提只加日誌、不改行為的 PR。
  不改行為的改動（日誌、註釋、文件）與有測試 / 回放兜底的 Core 邏輯不受此限。
- 主題與自繪渲染器（`crates/qingjian-render`、各殼的貼圖路徑、主題檔案）還在測試，這部分暫不接受 PR；穩定一版後再開。
