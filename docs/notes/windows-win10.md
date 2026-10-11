# Windows 10 上的設定程式（2026-09-13）

## 起因

Windows 10 22H2 上點開始選單的「青簡設定」，彈的是系統錯誤框：

> qingjian-settings.exe - 無法找到入口
> 無法定位程式輸入點 TryCreatePackageDependency 於動態連結庫 C:\Program Files\Qingjian\qingjian-settings.exe 上。

同一臺機器上 Server 與 TSF DLL 一切正常（候選、上屏、學習都在跑），只有設定程式起不來。

## 定位

1. 讀 `qingjian-settings.exe` 的 PE 匯入表：`api-ms-win-appmodel-runtime-l1-1-5.dll` 下掛著
   `TryCreatePackageDependency` 與 `AddPackageDependency`，而且是**靜態匯入**（延遲匯入段是空的）。
2. 在 Windows 10 22H2（19045.5011）上逐個探匯出：`kernel32.dll` / `kernelbase.dll` / `AppXDeploymentClient.dll` /
   `Kernel.Appcore.dll` 與 `api-ms-win-appmodel-runtime-l1-1-{0,1,2,3}` 全都沒有這兩個函式，符號名在 System32 裡
   也搜不到。微軟文件寫明 `TryCreatePackageDependency` 的 Minimum supported client 是
   **Windows 11（10.0.22000）**——Windows 10 的任何版本都沒有這個 API，也沒有可再發行元件能補上
   （補 DLL 那一類野路子對它無效）。
3. 名字只能來自依賴：`apps/windows/settings` 用 `windows-reactor = "0.100"`，它的
   `src/native/winui/bindings.rs` 用 `windows_core::link!("api-ms-win-appmodel-runtime-l1-1-5.dll" …)` 早期繫結
   這兩個函式，`src/native/winui/bootstrap.rs` 拿它們把 `Microsoft.WindowsAppRuntime.2` 框架包加進程序包圖
   （框架依賴部署的標準做法，但那條路只有 Windows 11 24H1+ 走得通）。
4. 上游有同樣的問題與討論：microsoft/windows-rs issue #4866（Reactor 5419f50 之後，Windows 10 上以
   0xc0000139 STATUS_ENTRYPOINT_NOT_FOUND 退出）、PR #4873（改成執行時解析；**已關閉未合併**，master 至今
   仍是早期繫結，0.100.0 也是）、issue #4692（自包含 + `bootstrap()` 會缺 `MddBootstrapInitialize2`）。
5. 關鍵事實：機器上裝的 Windows App Runtime **自己**的 MSIX 清單寫的是
   `<TargetDeviceFamily Name="Windows.Universal" MinVersion="10.0.17763.0" />`——執行時支援 Windows 10 1809+，
   不支援的只是 Reactor 的引導**方式**。

## 結論

- 框架依賴部署在 Windows 10 上無解：它必須先靠那兩個 Windows 11 才有的 API 把框架包加進程序包圖。
- **自包含部署**（exe 旁邊自帶一份 Windows App Runtime）在 Windows 10 上可行：Reactor 見到清單裡有
  `windows-reactor-self-contained` 標記、且 exe 旁邊有 `Microsoft.WindowsAppRuntime.dll`，就整段跳過引導。
- 但光換部署方式不夠：IAT 裡那兩個名字會讓 Windows 10 在**載入期**就拒絕啟動，連自包含分支都進不去。

## 改法

- `apps/windows/settings/build.rs`：① `windows_reactor_setup::as_self_contained()` 鋪執行時並嵌自包含清單；
  ② 連結引數加 `/DELAYLOAD:api-ms-win-appmodel-runtime-l1-1-5.dll` + `delayimp.lib`，把這兩個名字從 IAT 挪到
  延遲載入描述符——自包含部署下它們永遠不會被呼叫，Windows 10 也就不會去找它們。
- `apps/windows/installer/`：`settings-runtime.txt` 記要跟著裝的執行時清單，`build.ps1` 按清單從
  `target\release` 挑進 `target\installer\settings-runtime`，`qingjian.iss` 整個目錄裝進 `{app}`。
  代價：安裝包多約 56 MB 的執行時（185 個檔案，LZMA2 壓完更小），換來的是不依賴機器上裝沒裝框架包。

## 驗證

在本機（Windows 10 22H2，19045.5011）跑：

```powershell
cargo build --release -p qingjian-windows-settings
target\release\qingjian-settings.exe
```

設定視窗應當正常開啟（不再彈入口錯誤），改一項設定能寫進 `%APPDATA%\Qingjian\config.toml`。
反證：把 `build.rs` 裡那兩行 `/DELAYLOAD` 去掉重編，同一臺機器上會重新以 0xc0000139 在載入期失敗。
CI 側：`windows` job 編三個 Windows 產物；`release.yml` 打 Inno 包時缺任何一項執行時檔案會在 `build.ps1` 裡直接失敗。

## 經驗

- 「無法定位程式輸入點 X 於**動態連結庫 <自己的 exe>** 上」是 API-set 早期繫結缺失時的典型措辭：報出來的模組名
  可能是宿主 exe，而不是真正缺函式的那份 DLL；順著 PE 匯入表找，比按報錯文案猜快得多。
- 依賴裡的 `#[link]` / `windows_core::link!` 會成為**你的** exe 的匯入表條目，哪怕那條程式碼路徑在執行時根本走不到。
  「上游 crate 支援多老的系統」與「你的 exe 能在多老的系統上**啟動**」是兩件事。
