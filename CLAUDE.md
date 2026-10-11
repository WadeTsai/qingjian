# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## 倉庫現狀

跨平臺輸入法，Core 平臺無關，各平臺只做殼。已發版 0.1.3（macOS 與 Windows 共用版本號，2026-09-17）；Linux 第一階段提供 Rust Server + Fcitx5 預設候選面板（原始碼安裝，0.1.4 起另有 Ubuntu 26.04 預編譯包）。
階段與已完成項見 `docs/plan/roadmap.md`，待辦見 `docs/plan/todo.md`。

## 目錄地圖

一行一個，只說它是什麼、入口在哪；實現要點（資料檔案、常數、生成命令）在 `docs/notes/crate-notes.md`，改了實現要同步那裡。

- `crates/qingjian-core`：引擎。`Engine` 是對外唯一門面，`Translator` / `Learner` 等 trait 在 `engine` 模組；拼音解析、糾錯、候選、排序、整句、雙拼、注音、英文模式都在這裡。
- `crates/qingjian-dictionary`：詞庫（TSV 或 `.qj` mmap），按音節位置二分查詢。
- `crates/qingjian-translate`：釋義表 `Glossary`、詞彙等級表 `LevelTable`。
- `crates/qingjian-learning`：使用者側落盤：詞頻 / 使用者詞 / 個人 n-gram / 敲錯表（`FrequencyLearner`）、輸入日誌（`InputLog`）、輸入統計（`UsageStats`）、詞彙記錄（`VocabularyBook`）。
- `crates/qingjian-predict`：雲聯想 `CloudPredictor`（OpenAI 相容介面）與釋義兜底 `CloudGlossFiller`；`PredictConfig` 是 `[predict]` 分節。
- `crates/qingjian-lm`：整句轉換的 bigram 語言模型 `BigramModel`。
- `crates/qingjian-neural`：字級 Transformer 本地推理 `CharScorer`（candle），給整句前幾條路徑重打分。
- `crates/qingjian-format`：`.qj` 資料容器（mmap 讀、零複製檢視、寫入器、雜湊索引）。
- `crates/qingjian-platform`：平臺層共用：`Config`（TOML 配置）、`extra_dictionaries`、Windows Server ↔ DLL 的 `protocol` 型別。
- `crates/qingjian-render`：自繪渲染器（spike 中，分支 renderer-spike）：候選窗一幀 + 主題 → 點陣圖，各平臺只貼圖。見 `docs/design/rendering.md`。
- `crates/qingjian-update`：檢查更新：讀官網的 `releases.json`、驗 ed25519 簽名、按平臺與渠道挑新版本，只提示不安裝；發版側的簽名工具是 `tools/release-sign`。見 `docs/design/update.md`。
- `apps/cli`：Core 的驗證工具：查詢、逐鍵計時、輸入日誌回放、整句評測、常數掃描。排序 / 整句 / 糾錯的改動先跑它再合。
- `apps/macos`：IMK 殼，按 `app / host / imk / candidates / menubar / preferences` 分目錄；`scripts/bundle.sh --install` 裝到本機，`--pkg` 出分發包。
- `apps/windows`：`server`（Server 程序：Engine + IPC + 自繪候選窗與狀態條）+ `tsf`（TSF DLL）+ `settings`（WinUI 3）+ `installer`（Inno）。DLL 不能帶 Engine 的依賴樹，所以是兩個 package。
- `apps/linux`：`server`（Engine + Unix socket）+ `fcitx5`（框架事件、預設面板）+ `scripts`（使用者級安裝解除安裝、`package.sh` 打預編譯包）；Server 手動啟動，見 `docs/notes/linux-fcitx5.md`。
- `tools/dict-convert`、`tools/gloss-gen`、`tools/corpus`：產品資料生成（詞庫 / 語言模型 / 釋義表 / emoji / 英文詞表），輸出到 `data/generated/`（gitignore）。
- `assets/`：隨包資料來源與樣例，各目錄有 README 寫來源與許可。霧凇拼音（GPL）已徹底移除，不要再引入。

`docs/` 分四類（索引在 `docs/README.md`）：`design/` 設計與決定、`plan/` 路線與待辦、`notes/` 工程記錄（效能、復盤、踩坑、crate 實現要點）、
`user/` 使用者文件（官網構建時拉取渲染，約定見 `docs/user/README.md`，措辭面向使用者、不出現實現詞）。

## 常用命令

```bash
cargo build                                   # 整個 workspace
cargo test                                    # 全部測試；-p <crate> 單個，加測試名過濾
cargo clippy --all-targets -- -D warnings
cargo fmt --all
cargo run -p qingjian-cli -- <拼音>...          # Core 的主要驗證方式；--replay / --eval-text 見 crate-notes
apps/macos/scripts/bundle.sh --install        # mac 殼裝到 ~/Library/Input Methods/（IMK 不能 cargo run 驗證）
```

Windows 本機只 `cargo check --target x86_64-pc-windows-gnu`，真編譯與真機測試在 Windows 機器上做（部署方式見 `apps/windows/README.md`）；
端到端驗證 mac 可用 `osascript` 往 TextEdit 發按鍵再讀迴文本。

## 約定

架構約束、程式碼組織、版本號、提交資訊、文件同步、提交前檢查與發版都在 `docs/contributing.md`，隨本檔案一起載入：

@docs/contributing.md

交流用中文。
