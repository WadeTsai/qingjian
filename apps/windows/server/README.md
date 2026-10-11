# qingjian-windows-server

青簡 Windows 輸入法的 **Server 程序**（bin `qingjian-server`）：持有唯一的輸入核心
`qingjian-core::Engine`，跑在所有應用程序之外，通過命名管道給 TSF DLL（`../tsf`）提供候選。
Windows 端的整體結構、為什麼核心要在程序外、構建與註冊步驟，見 `../README.md`。

## 模組

- `assembly`：裝配 Engine（詞庫 / 釋義 / 學習）。
- `dispatch::Router`：按 `SessionId` 分派多會話，處理按鍵、上屏、非同步結果推送。
- `session`：單個應用會話的組句狀態。
- `ipc`：長度字首幀的收發迴圈；`ipc::pipe`（`cfg(windows)`）在 `\\.\pipe\qingjian` 上起命名管道服務。

bin `src/main.rs` 只做配置讀取、Engine 裝配與啟動；邏輯都在庫部分，`tests/` 裡的整合測試直接用庫。
