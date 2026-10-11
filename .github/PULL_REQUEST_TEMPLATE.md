<!-- 謝謝投稿。下面的清單是合併前維護者會逐條看的，提前對一遍能少一輪往返；不適用的項劃掉即可。 -->

## 改了什麼

<!-- 一兩句：解決什麼問題、怎麼解的。關聯的 issue 寫「關閉 #編號」。 -->

## 平臺

- [ ] macOS
- [ ] Windows
- [ ] Linux
- [ ] iOS
- [ ] Android
- [ ] 與平臺無關（Core / 資料 / 文件）

## 合併前清單

- [ ] `cargo fmt --all --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` 本機全過
- [ ] **使用者能感知的行為變了（按鍵、候選、選單、設定項、配置檔案、資料檔案），`docs/user/` 對應頁面已同步改**，按鍵改動同時更新 `docs/user/getting-started/keys.md`
- [ ] 一個功能只在一個平臺實現時，已在文件裡標明平臺，並在 PR 裡說明另一平臺的差距
- [ ] 程式碼識別符號英文、註釋與文件中文；新檔案有 `//!` 檔案頭；單檔案不超過 800 行
- [ ] 提交資訊用 Conventional Commits（`fix(core): ……` / `feat(windows): ……`，說明寫中文）；其餘約定見 [docs/contributing.md](../docs/contributing.md)
- [ ] 不改 `CHANGELOG.md`（發版時由維護者統一寫）

## 怎麼驗證的

<!-- 修 bug、改按鍵 / 上屏 / 候選窗位置的 PR 必填：在自己機器上覆現過、改完在同一個應用裡確認修好，寫明系統版本（Windows 註明 10 還是 11）、應用、操作步驟，可附截圖或錄屏。
     編譯與 CI 通過不算驗證；復現不了的請把分析寫在 issue 裡，或只提加日誌的 PR。 -->
