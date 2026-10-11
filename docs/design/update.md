# 檢查更新

2026-09-17 定。0.1.3 只做「檢查 + 提示 + 開啟下載頁」，下載與安裝留給後面的版本。

## 目標與邊界

- 使用者不用再從群裡拿包：發了新版，已裝的青簡一天內自己知道。測試者把渠道切到「測試版」就能收到 beta / rc。
- 輸入法不替換自己：只提示，點了開啟 [下載頁](https://qingjian.app/download)，安裝仍由系統安裝程式完成。
- 更新通道等於遠端程式碼執行通道，所以索引從第一版起就簽名；客戶端只認簽過名的索引。
- 隱私：請求是對靜態檔案的 GET，不帶賬號、裝置標識或輸入內容；`[update] check = false` 關掉後不聯網。

## 資料流

```
發版（release.yml）
  releases_json.py → releases.json
  qingjian-release-sign sign → releases.json.sig      私鑰：CI 金鑰 QINGJIAN_INDEX_SIGNING_KEY
  兩個檔案掛到本次 Release 與 GitHub latest
官網構建（qingjian-web scripts/sync-releases.mjs）
  原樣拷到 static/ → https://qingjian.app/releases.json 與 .sig
客戶端（crates/qingjian-update）
  下載兩者 → ed25519 驗籤 → 解析 → 按平臺 / CPU / 渠道挑比當前新的最新版 → 寫 update.json
```

索引放官網域名而不是直連 GitHub：國內訪問 GitHub 不穩，官網是 Cloudflare 上的靜態站（非瀏覽器 User-Agent 實測不被攔）。
簽名是對檔案位元組的分離簽名，所以官網必須原樣複製，不能經 JSON 重排。

## 渠道與版本

- 索引裡每個版本的 `channel` 是更新渠道：定期發的版本 `stable`，中間放給測試者的 `alpha` / `beta` / `rc`（版本號帶同名預釋出字尾）。
- 客戶端兩檔：`stable` 只看 `stable`；`beta` 看全部，取版本號最大的，所以測試版使用者在正式版出來後會升到正式版。
- 版本按語義化版本比較（`0.1.4-beta.2 < 0.1.4-rc.1 < 0.1.4`）。
- 本地開發包（`0.1.3-dev-<雜湊>`）不檢查。除錯用環境變數：`QINGJIAN_UPDATE_VERSION` 頂替當前版本，`QINGJIAN_UPDATE_INDEX` 換索引地址（簽名照驗）；
  `cargo run -p qingjian-update --example check -- 0.1.2 beta` 手動走一遍。
- 只有帶本機安裝包（`platform` + `cpu`）的版本才算數：只發了 macOS 的版本不會提示 Windows 使用者。

## 排程

殼在已有的每秒定時器裡調 `Checker::poll`：開關開著、上次成功超過 24 小時（或換了渠道）、上次嘗試超過 1 小時、沒有正在查的，才起一個一次性執行緒去查。
失敗（斷網、驗籤不過、格式版本不認識）只記日誌。結果落在資料目錄的 `update.json`，裝上新版後舊結果因為「不比當前新」自然失效。

- macOS 選單的坑（2026-09-17）：「有新版本」這一行是可點的條目，必須放在「開啟日誌目錄」那一組裡。第一版把它放在最後一條分隔線之後、
  夾在「配置檔案有錯誤」與「青簡 x.y.z」兩個純展示條目之間，結果 IMK 每次按鍵後都 `deactivateServer` 再新建控制器，
  第二個字母蓋掉第一個、候選框閃；與條目是否隱藏、標題、tag 都無關，挪個位置或去掉 action 就好（對照構建逐個驗過）。
- macOS：IMK 程序自己查；選單裡「有新版本 x.y.z…」一行與「關於」頁的狀態行，狀態變了才刷介面。
- Windows：Server 查並寫 `update.json`（DLL 不聯網）；設定程式的「關於」頁讀這個檔案，「立即檢查」在設定程式自己的後臺執行緒裡查。
  工作列「中 / 英」圖示的右鍵選單在查到新版本時多一項「有新版本，前往下載…」：Server 隨 `ModeSync` 下發 `IndicatorState.update_available`，
  點了發 `IndicatorCommand::OpenDownload`，由 Server 開啟下載頁（DLL 可能在 UWP 沙箱裡起不了程序）。

## 簽名金鑰

`tools/release-sign`：`keygen --out <檔案>` 生成金鑰對（私鑰只寫檔案），`sign <檔案>` 按環境變數裡的私鑰寫 `.sig`，`verify <檔案>` 按內建公鑰驗。
公鑰在 `crates/qingjian-update/src/index/signature.rs` 的 `PUBLIC_KEYS`，是個列表：換鑰時新舊並列發一兩個版本，等舊版本使用者都升上來再去掉舊的。
`sign` 會先核對私鑰與內建公鑰對得上，對不上當場失敗；`release.yml` 的門禁在沒配私鑰時直接失敗——籤不了名的索引等於所有使用者收不到更新。

## 以後

- 下載 + 校驗 sha256（索引裡已有）+ 調起安裝程式（macOS 開啟 pkg，Windows 靜默跑安裝包）。
- 系統通知裡的提示。
- SignPath 要求「向非使用者指定的系統發資料」的功能在安裝時可關：安裝嚮導里加一個「自動檢查更新」勾選項。
