# 青簡 Windows 輸入法

Windows 端是**一個產品、兩個產物**，各自一個 package，同放本目錄：

| 目錄 | package | 產物 | 職責 |
| --- | --- | --- | --- |
| `server/` | `qingjian-windows-server` | `qingjian-server.exe` | 持有唯一的輸入核心 `qingjian-core::Engine`，跑在所有應用程序之外 |
| `tsf/` | `qingjian-windows-tsf` | `qingjian_tsf.dll` | TSF 文本服務，被載入進每個應用程序，只做按鍵轉發與文件寫入（候選視窗由 Server 自繪） |

```
應用程序 A ── qingjian_tsf.dll ─┐
應用程序 B ── qingjian_tsf.dll ─┼─ 命名管道 \\.\pipe\qingjian ─▶ qingjian-server（唯一的 Engine）
應用程序 C ── qingjian_tsf.dll ─┘
```

## 為什麼核心邏輯要在程序外

Windows 的文本服務（TSF，Text Services Framework）是一個 COM DLL（`ITfTextInputProcessor`），
系統會把它載入進**每一個**接受文本輸入的應用程序。因此輸入核心不能待在 DLL 裡（會被複制進幾十個程序、
狀態無法共享、崩潰會連累宿主應用）。青簡照 Weasel（WeaselServer + WeaselTSF）、水杉（Server 程序）的做法：
Engine 只此一份，跑在獨立的 Server 程序；每個應用程序裡的 TSF DLL 只做兩件事——把系統按鍵翻成協議訊息發來、
把 Server 回的候選畫到候選視窗。

## 為什麼是兩個 package 而不是一個

兩個產物的依賴集合刻意不同：DLL 只依賴 `qingjian-core`、`qingjian-platform` 與官方 `windows` crate（COM
`implement` 宏），Server 才依賴詞庫 / 學習 / 翻譯 / 雲聯想（含 tokio）整棵樹。合成一個 package 後，DLL 的
編譯單元會拉進 Server 的依賴；用 feature 區分也不行，workspace 一起構建時 feature 會統一。crate 邊界就是
「DLL 不含 Engine」這條約束的強制手段。判斷標準與 macOS 的 IMK 殼一致：換掉平臺適配層，不應該需要改
Core 的任何一行。

## 三個部分

- **Server 程序**（`server/`）：裝配並持有 Engine（詞庫 / 語言模型 / 翻譯 / 學習），按 `SessionId` 為每個
  應用會話維護各自的組句狀態，處理按鍵、產出候選與上屏文本，把雲聯想 / 本地整句模型的非同步結果主動推給
  對應會話。
- **IPC 協議**（`qingjian-platform::protocol`，兩端共用）：`ClientMessage`（DLL → Server：開 / 關會話（帶宿主 exe 名，
  `[apps]` 按應用設定據此查）、按鍵、上屏、回上下文）與 `ServerMessage`（Server → DLL：按鍵結果、上屏結果、非同步重繪、請求上下文），一次要繪製的狀態是
  `Frame`（preedit 分段 + 候選頁）。長度字首 JSON 幀的編解碼與預設管道名也在這裡，DLL 不必依賴整個 Server 庫。
- **TSF DLL**（`tsf/`）：分「引擎層」`client`（平臺無關的管道客戶端 `EngineClient`，泛型在任意 `Read + Write`
  上，本機就能接真 Server 端到端測）與「COM 層」`com`（`cfg(windows)`：`DllGetClassObject` → `IClassFactory`
  → `#[implement(ITfTextInputProcessor, ITfKeyEventSink)]`，編輯會話上屏，把組句位置報給 Server 擺候選視窗，
  `DllRegisterServer` 註冊文本服務）。

設計細節見 `docs/design/architecture.md`「Windows：TSF」。

## 構建

本機（macOS / Linux）只做交叉 `check`，產出不了可用二進位制，但協議層的端到端測試能跑：

```bash
cargo check --target x86_64-pc-windows-gnu -p qingjian-windows-server -p qingjian-windows-tsf
cargo test -p qingjian-windows-server -p qingjian-windows-tsf
```

真正編譯與試用都在 Windows 機器上（MSVC 工具鏈）。

### 本地聯調：`scripts/test-local.ps1`

改了 Server 或 DLL 之後要試手，用這個指令碼，別手敲：

```powershell
pwsh -File apps\windows\scripts\test-local.ps1            # 編譯 + 換 Server + 起，DLL 那步要管理員
pwsh -File apps\windows\scripts\test-local.ps1 -SkipBuild -SkipRegister
```

它做四件事：編 Server 與 TSF DLL（帶 `QINGJIAN_UIACCESS=0`，沒簽名的 exe 帶 uiAccess 起不來）、
停掉在跑的 Server、把**已安裝目錄整份拷到 `~\qingjian-devtest`** 再換上新的 Server 與隨包資料、
起 Server 並列印剩下要手動做的事（註冊 DLL、設方案、敲哪幾組鍵）。

> **指令碼存成帶 BOM 的 UTF-8**（`sign-local.ps1` 也是）。Windows PowerShell 5.1 對沒有 BOM 的 `.ps1`
> 按系統 ANSI 碼頁解析，中文會變成亂碼、引號配對跟著崩，報出來的卻是「字串缺少終止符」這種語法錯。
> 編輯時別把 BOM 去掉。PS7 兩種都讀得對，所以只用 `pwsh` 驗會漏掉這個問題。

**為什麼不在倉庫裡直接跑**：`bundled_root()` 按 exe 位置找資料，`target\debug\` 下會落到倉庫根，
而 `data\generated\` 是 gitignore 的、本機多半沒有，於是退回 `assets\sample\` 樣例——形碼候選照樣出得來
（碼錶獨立），但譯文幾乎全空，會讓人誤以為釋義那條設計沒生效。

**為什麼兩邊要一起換**：`qingjian_core::Candidate` 是線上格式的一部分（見 `protocol/mod.rs`）。
Core 加一個 `CandidateKind` 變體，老 DLL 就解不出整條幀、把按鍵原樣放行——表現是「輸入法突然只出英文」，
日誌裡只有一句 `unknown variant`。所以 Server 與 DLL 必須同一份原始碼編出來的；協議版本號對不上時
Server 會記警告，但**它只警告、不拒絕**，別指望它兜住。

### 手工步驟（腳本里也在做，供對照）

```bat
:: 1) 編譯出 DLL 與 Server
cargo build -p qingjian-windows-tsf -p qingjian-windows-server

:: 2) 註冊文本服務（改 HKEY_CLASSES_ROOT，圖示寫到 %ProgramData%\Qingjian\qingjian.ico，要管理員）
regsvr32 target\debug\qingjian_tsf.dll

:: 3) 起 Server（引擎在這裡；沒起時 DLL 吃掉字母鍵但沒有候選，起來後下一鍵 / 下次聚焦自動重連）
cargo run -p qingjian-windows-server

:: 4) 在系統「語言 / 輸入法」裡應能看到「青簡」（中文(簡體) 下）與「青簡」「青簡注音」（中文(台灣) 下，裝了這個語言才顯示；預設出繁體，後者按注音解鍵），
::    切到它，在任意輸入框敲字
::    日誌都在 %LOCALAPPDATA%\Qingjian\logs\：server.<日期>.log / tsf.<日期>.log / settings.<日期>.log（按天，留 7 天）

:: 反註冊
regsvr32 /u target\debug\qingjian_tsf.dll
```

## 設定程式與 Windows App Runtime

`settings/`（`qingjian-settings.exe`）用 Windows Reactor（WinUI 3）畫介面，是三個產物裡唯一依賴 Windows App Runtime 的。
它的部署方式是**自包含**：`build.rs` 讓 `windows-reactor-setup` 把 `Microsoft.WindowsAppSDK.Runtime` 的 MSIX 解到
`target\release\` 並按自包含標記嵌清單，安裝包把這些檔案裝到 exe 同級——不依賴機器上裝沒裝框架包。
Windows 10 上框架依賴的引導走不通（它要先調 Windows 11 才有的 `TryCreatePackageDependency`），
同一個 `build.rs` 還把這兩個 API 改成延遲載入：否則它們會進 exe 的匯入表，Windows 10 在載入期就起不來。
定位過程、上游 issue / PR 與取捨見 `docs/notes/windows-win10.md`；打包側見 `apps/windows/installer/README.md`。

## 版本與釋出

各平臺殼版本號獨立（見 `docs/notes/release.md`）：兩個 package 的 `version` 各自寫在自己的 `Cargo.toml`，
不跟 workspace 走；釋出標籤用 `windows-v<版本>`。
