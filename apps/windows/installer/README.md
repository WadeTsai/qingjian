# 青簡 Windows 安裝包

用 [Inno Setup](https://jrsoftware.org/isinfo.php) 打的安裝包，把 TSF DLL（64 位與 32 位各一份）、Server、設定程式與隨包資料一起裝進
`C:\Program Files\Qingjian`，註冊文本服務，並設登入自啟。對應 macOS 的 pkg。

## 安裝佈局

```
C:\Program Files\Qingjian\
    qingjian_tsf-<版本>.dll       TSF 文本服務（64 位；被載入進每個應用程序；按版本起名，見「升級」）
    qingjian_tsf-<版本>-x86.dll   同上的 32 位版（企業微信 / WPS / 32 位 QQ 這類 32 位應用只能載入它）
    qingjian-server.exe       輸入核心 Server（跑在應用程序外）
    qingjian-settings.exe     設定介面
    Microsoft.UI.Xaml.dll …   設定程式自帶的 Windows App Runtime（自包含部署，見下節；約 56 MB / 185 個檔案）
    qingjian.ico              開始選單 / 啟動項快捷方式的圖示（exe 裡也嵌了一份）
    data\generated\           dict.qj / lm.qj / glossary-{en,ja,zh,es,vi}.qj / english.tsv / dicts\*.qj
    data\generated\codes\     隨包輔碼碼錶 stroke.qj + LICENSE-CNS11643.txt（tools\dict-convert 的 pack codes 生成）
    assets\                   emoji\ levels\ sample\
```

Server 與設定程式按 **exe 相對**定位隨包資源（`qingjian_platform::resources`）：裝機時資源與 exe 同級，
開發時是倉庫 `ime\`（exe 在 `target\{debug,release}\` 下往上三層）。相對寫法兩套佈局一致，只有根不同。

使用者資料仍在 `%APPDATA%\Qingjian`（config.toml、金鑰 .env、學習資料、統計），三個程序的日誌在 `%LOCALAPPDATA%\Qingjian\logs`（`server.` / `tsf.` / `settings.` 字首，按天，留 7 天）；
解除安裝不動這些。圖示由 `regsvr32` 寫到 `%ProgramData%\Qingjian\qingjian.ico`（DLL 裡 include_bytes 內嵌）。

## 安裝程式做的幾件事

1. **結束舊程序**：`PrepareToInstall` 裡 `taskkill` Server 與設定程式（只有這兩個 exe 要覆蓋）。
2. **應用容器許可權**：`icacls` 給安裝目錄加 `ALL APPLICATION PACKAGES`（SID `*S-1-15-2-1`）讀+執行。
   不加的話 UWP/AppContainer 應用（工作列搜尋、設定）讀不到 DLL，切不到青簡。
3. **註冊文本服務**：64 位 DLL 用 `regsvr32`、32 位 DLL 用 `SysWOW64\regsvr32`，各註冊一次（各自寫進自己檢視的 HKCR，`CTF\TIP` 兩邊共用；要管理員——安裝程式本就提權）。
   DLL 自己把語言 profile 登記在「中文(簡體)」與「中文(台灣)」兩種語言下（後者有「青簡」與「青簡注音」兩個），繁體中文系統的使用者在自己的語言下就能加上。
   安裝程式介面隨系統語言顯示簡體或繁體（`[Languages]` 兩條；快捷方式與啟動項名字不隨語言變，免得升級時留下兩套）。
4. **清舊 DLL**：裝完刪歷次版本留下的 `qingjian_tsf*.dll`，仍被應用佔用的登記成重啟後刪（`RestartReplace`）。
5. **登入自啟**：「啟動」資料夾放 Server 快捷方式（Explorer 走 ShellExecute 拉起才拿到 uiAccess；計劃任務拿不到）。
6. **立即啟動**：完成頁以當前非提升使用者 ShellExecute 起一次 Server，裝完就能用，不必先登出。

解除安裝反向：殺 Server / 設定程式 → 反註冊當前版本 DLL → 刪檔案（佔用中的 DLL 重啟後刪，`[UninstallDelete]` 兜住舊版本的）。

## 升級：DLL 被佔用怎麼辦

`qingjian_tsf.dll` 被載入進每一個有文本框的應用程序，檔案鎖著覆蓋不了；Inno 預設的 `CloseApplications`
用 Restart Manager 找出所有佔用者要求關閉——對輸入法 DLL 就是「關掉一切」，所以關掉它（`CloseApplications=no`），改成：

- DLL **按版本起名並排裝**（`DestName: qingjian_tsf-<版本>.dll`），新檔案從不與舊檔案撞名；
- 只 `regsvr32` 新檔案（InprocServer32 指向它）。**不要**對舊 DLL `regsvr32 /u`：那會把整個 CLSID / profile 登出掉；
- 已開著的應用繼續用程序裡的舊 DLL 直到重啟，Server 兩個版本都服務（`OpenSession` 帶協議版本，對不上只記警告）；
- 裝完刪舊 DLL，刪不掉的登記成重啟後刪。

## 設定程式自帶 Windows App Runtime

設定介面用 Windows Reactor（WinUI 3）寫，而它的框架依賴引導只有 Windows 11 走得通：要 Windows 11 才有的
AppModel API 把框架包加進程序包圖，Windows 10 上沒有那兩個函式（定位見 `docs\notes\windows-win10.md`）。
所以設定程式用**自包含部署**——`apps\windows\settings\build.rs` 讓 `windows-reactor-setup` 把 Windows App Runtime
鋪到 `target\release\`，打包時按 `settings-runtime.txt` 挑進 `target\installer\settings-runtime`，本目錄的
`qingjian.iss` 再整個目錄裝到 `{app}` 下、與 `qingjian-settings.exe` 同級。

- 這些檔案是執行時必需：少一件（或層級裝錯）設定視窗就起不來，`build.ps1` 發現缺檔案會直接失敗。
- 升級 `windows-reactor` / `windows-reactor-setup` 時，照新版 crate 的 `assets/runtime.txt` 核對 `settings-runtime.txt`。
- Server 與 TSF DLL 不依賴它；裝機體積的大頭仍是隨包資料。
- `windows-reactor-setup` 在 `cargo build` 時用系統 `curl.exe` 從 NuGet 下執行時包（無校驗，失敗只打印），快取在 `%LOCALAPPDATA%\windows-reactor-setup`；CI 的 runner 每次都會重下一遍。下載失敗的後果由 `build.ps1` 的缺項檢查兜住。

## 打包（在編譯機上）

```powershell
# 需要 MSVC 工具鏈 + Inno Setup。資料取自倉庫 data\generated 與 assets，打包前先確保 .qj 是最新的。
powershell -ExecutionPolicy Bypass -File apps\windows\installer\build.ps1
```

指令碼 release 構建三個產物、從 `apps\windows\server\Cargo.toml` 讀版本、找 `ISCC.exe`、編 `qingjian.iss`，
成品在 `target\installer\qingjian-<版本>-windows-x86_64-setup.exe`。改了資料 / 指令碼但二進位制沒變時加 `-SkipBuild`；`-Sign` 用自簽證書籤產物
（uiAccess 要求 Server 簽名 + 裝 Program Files）。

也可手動：`iscc /DAppVersion=0.1.0 apps\windows\installer\qingjian.iss`。

## 注意

- **Inno 版本**：開發機與 CI 統一用 Inno Setup **7.1.0**（CI 從 jrsoftware/issrc 的 GitHub Release 釘死下載）。它自帶簡體中文翻譯；
  6.x 的安裝包不帶 `Languages\ChineseSimplified.isl`，Chocolatey 也只有 6.x，別用。`ArchitecturesAllowed=x64compatible` 需 6.3+。
- **簽名**：發版證書就緒後在這裡加 `SignTool`（對應 mac 的 Developer ID）；開發期用 `-Sign` 的自簽證書。
- **沒證書的包**（CI 內測版）：先設 `$env:QINGJIAN_UIACCESS = '0'` 再打，Server 不嵌 uiAccess——沒簽名的 exe 帶 uiAccess=true 會起不來。
  代價是候選窗在 UWP 宿主裡可能被蓋住。`release.yml` 的 `windows` job 就是這麼打的。
