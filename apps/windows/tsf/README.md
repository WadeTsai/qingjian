# qingjian-windows-tsf

青簡 Windows 輸入法的 **TSF 文本服務 DLL**（產物 `qingjian_tsf.dll`）。Windows 會把它載入進每一個接受
文本輸入的應用程序；它只做適配，不含任何輸入邏輯——把系統按鍵翻成協議訊息發給獨立的
`qingjian-server` 程序（`../server`），再把 Server 回的候選畫出來。
Windows 端的整體結構、為什麼是兩個 package、構建與註冊步驟，見 `../README.md`。

## 兩層

- **引擎層**（`client`，平臺無關）：`EngineClient` 把開 / 關會話、按鍵、上屏編排成
  `qingjian-platform::protocol` 的訊息，在一條雙工位元組流上收發。它泛型在任意 `Read + Write` 上，所以能用
  `UnixStream::pair` 接上真正的 Server 端到端測（`tests/protocol_loop.rs`，本機就能跑）。
  `cfg(windows)` 的 `client::pipe` 用 `std::fs::File` 開啟命名管道當這條流。
- **COM 層**（`com`，`cfg(windows)`）：實現 TSF 要求的 COM 介面。`DllGetClassObject` → `IClassFactory` →
  `#[implement(ITfTextInputProcessor, ITfKeyEventSink)]`；`Activate` 時把自己掛到擊鍵管理器上收鍵、並連
  Server；`OnKeyDown` 轉發按鍵，`edit_session` / `composition` 經 `ITfContext` 做 preedit 內聯與上屏，
  `anchor` 把組句 / 選區的螢幕矩形報給 Server 擺候選視窗（視窗在 Server 程序自繪），`poll` 定時拉取雲聯想的非同步
  結果。`DllRegisterServer`（`registry`）寫 InprocServer32 並經 `ITfInputProcessorProfiles` /
  `ITfCategoryMgr` 把青簡登記成鍵盤類文本服務，語言 profile 在「中文(簡體)」下一個、「中文(台灣)」下兩個（拼音的「青簡」與注音的「青簡注音」，`com/profile/language.rs`），
  `com/profile` 查當前啟用的是哪個並把簡 / 繁、注音與否報給 Server。

DLL 側日誌在 `%LOCALAPPDATA%\Qingjian\logs\tsf.<日期>.log`（與 Server / 設定程式同目錄，字首區分），按天一個檔案、只留最近 7 天。
