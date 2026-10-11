# 跨平臺 UI 與自繪渲染器（2026-09-13）

## 起因

主題功能提上日程，同時有人建議用 Flutter 一類跨平臺框架，「一套 UI 跑 Windows / macOS / Linux / iOS / Android」。
這篇記錄調研結論與選定的方向：**候選窗、狀態條、手機鍵盤這類「顯示面」用一個 Rust 自繪渲染器輸出點陣圖，各平臺只貼點陣圖；
設定程式繼續用各平臺原生 UI，不自繪。** 在此之前定過「Windows 候選窗改 Direct2D + DirectWrite」，被本方向取代（見末節）。

## 輸入法的 UI 分兩種

| 種類 | 例子 | 要求 | 結論 |
|---|---|---|---|
| 顯示面 | 候選窗、懸浮狀態條、手機鍵盤面板 | 無邊框、不搶焦點、置頂、按鍵後毫秒級出現；輸入只有點選與拖拽 | 只畫矩形 / 陰影 / 漸變 / 文字，適合自繪 |
| 控制元件面 | 設定程式 | 表單、表格、滾動、文本框、無障礙 | 需要完整控制元件庫，自繪等於寫一個 toolkit，不做 |

現有程式碼量（2026-09-13）：Core 15,655 行；macOS 候選窗 + 選單欄 1,644 行、偏好設定 3,041 行；Windows 候選窗 + 狀態條 + 托盤 2,057 行、設定程式 1,491 行。
平臺層不到總量兩成，Core 已經是共享的那「一套」，能收攏的只有顯示面的繪製部分。

## 調研（2026-09-13）

- **iOS 鍵盤擴充套件記憶體上限**：社群實測 48–60 MB（phys_footprint），超出由 jetsam 靜默殺掉、無崩潰日誌、切回系統鍵盤。
  手機端的攔路虎是記憶體不是 UI：隨包 56 MB 模型與詞庫必須裁剪；任何帶引擎的 UI 框架都塞不進去。
  來源：[The three hard constraints of an iOS keyboard extension](https://dev.to/tbds_2dadf2b626f315902eae/the-three-hard-constraints-of-an-ios-keyboard-extension-46af)、
  [react-native #31910（48 MB 崩潰）](https://github.com/facebook/react-native/issues/31910)。
- **Flutter 進 iOS 擴充套件**：傘狀 issue [flutter #124287](https://github.com/flutter/flutter/issues/124287) 的 `app_extension_safe` 引擎標誌仍在開發，
  鍵盤擴充套件 [flutter #59753](https://github.com/flutter/flutter/issues/59753) 以連結錯誤收場；[官方文件](https://docs.flutter.dev/platform-integration/ios/app-extensions) 只覆蓋 Share 一類擴充套件。今天不可用。
- **Flutter 桌面視窗控制**：無邊框仍是開放需求 [flutter #71042](https://github.com/flutter/flutter/issues/71042)，透明 / 置頂靠
  [window_manager](https://github.com/leanflutter/window_manager)、flutter_acrylic 這類外掛，Windows 上走未公開的 `SetWindowCompositionAttribute`（[flutter #96732](https://github.com/flutter/flutter/issues/96732)）。能做但脆。
- **Avalonia（C#，Skia 自繪）**：`ShowActivated=false` + `Topmost` + `TransparencyLevelHint` 是一等屬性，Windows / macOS 能做不搶焦點的置頂窗
  （[討論 #15805](https://github.com/AvaloniaUI/Avalonia/discussions/15805)）；Linux 上 `ShowActivated` 失效（[#17186](https://github.com/AvaloniaUI/Avalonia/issues/17186)）。
  是排除 WebView 之後最順的現成框架，但要常駐 .NET 程序（NativeAOT 後仍幾十 MB）、macOS 要改成程序外渲染、手機鍵盤進不去。
- **WebView（Tauri 等）**：已排除，各平臺 WebView 渲染不一致，且 Windows 依賴 WebView2 執行時（水杉 issue #68 就是缺它）。
- **Linux**：輸入法是 Fcitx5 / IBus 的外掛，候選窗由框架畫，我們只出引擎，顯示面幾乎為零。
- **一個被修正的判斷**：「候選窗必須原生」說得太死。Windows 候選窗已經由 Server 程序畫、DLL 走 IPC 發幀，
  啟動成本只付一次，延遲就是那趟 IPC；所以桌面端「常駐程序 + 自繪」在架構上成立。自繪渲染器正是利用這一點，而且不需要引入框架。

## 主題的兩條路

主題 = 顏色、字型、圓角、陰影、透明、漸變、佈局。

1. **共用主題模型 + 各平臺各自的渲染器**（Rime：一份 YAML，Squirrel 與 Weasel 各畫各的）。已知痛點是特性長期不對齊。原「D2D 重寫」屬於這條。
2. **一個渲染器**。顯示面一套程式碼、畫素級一致，主題只做一次，使用者可以自己寫主題檔案分享。

選第 2 條。

## 方案

一個 Rust crate 負責佈局 + 繪製，輸入是 Core 給的 `Frame`（候選、拼音行、譯文、分頁、提示）與主題資料，輸出一張 BGRA 點陣圖與命中區域表；
各平臺只做兩件事：把點陣圖貼上視窗、把點選 / 拖拽座標傳回來。

| 平臺 | 貼點陣圖的方式 | 改動 |
|---|---|---|
| Windows | 分層視窗 `UpdateLayeredWindow` 本來就吃點陣圖，`server/src/ui/layered/` 的合成器換成貼渲染器輸出 | 比 D2D 改動更小 |
| macOS | NSPanel 上一個 CALayer，`contents` 設 CGImage；視窗層級 / Space 處理不變 | 幾十行 |
| Linux | 不用，Fcitx5 自己畫 | 無 |
| iOS / Android | 鍵盤 View 貼點陣圖，命中測試在 Rust 裡做 | 自繪點陣圖佔幾 MB，符合記憶體上限 |

技術選型：

- 2D 柵格 `tiny-skia`：純 Rust，CPU。候選窗 2 倍屏幾百乘幾百畫素，亞毫秒；不上 GPU（vello / wgpu 對這個尺寸是負擔）。
- 文字 `cosmic-text`：fontdb 找字型 + rustybuzz 整形 + swash 柵格，帶回退鏈與彩色 emoji。**這是唯一的技術風險點。**
- 主題檔案 TOML，隨包給內建主題，使用者目錄可加。

## spike：先驗證再投入

在分支 `renderer-spike` 上做，目標是畫一行「青簡 hello 🙂 日本語」加圓角與柔和陰影，兩個平臺截圖與原生並排比較。要回答四個問題：

1. **彩色 emoji**：Apple Color Emoji 是 sbix 點陣圖，Segoe UI Emoji 是 COLRv0，swash 聲稱都支援，眼見為實。
2. **中日字形回退**：Han 統一編碼下必須帶 locale，否則 macOS 上會拿 Hiragino 給中文畫日式字形；確認 cosmic-text 按 zh-CN / ja-JP 選到 PingFang / 微軟雅黑 / Hiragino / Yu Gothic。
3. **字型庫載入**：fontdb 掃全系統字型幾百毫秒、幾十 MB，要改成按需載入幾個家族；手機上尤其。
4. **文字觀感**：Windows 上是灰度抗鋸齒沒有 ClearType，與原生並排看能否接受（Flutter、部分配置的 Chrome 也是灰度）。

驗收：四條都過，且首幀渲染耗時與記憶體不劣於現有 GDI / AppKit 實現，就把 Windows 與 macOS 一起換到渲染器；任一條不過，退回「各平臺各自渲染」（Windows 走 D2D）。

## spike 結果（2026-09-13 晚，macOS）

程式碼：`crates/qingjian-render`（渲染器）、`examples/preview.rs`（離線出 PNG + 量寬度）、`apps/macos/src/candidates/bitmap/`（殼側貼點陣圖；`[general] renderer = "system"` 切回 AppKit 繪製，偏好設定「候選視窗」頁可選，是過渡期退路，穩定一個版本後刪）。
對比方法：TextEdit 裡敲 `nihao`，`screencapture -l` 抓真實候選窗；同一幀人工抄進樣例，渲染成 PNG 並排；再把渲染器裝進殼抓真機。

| 驗收 | 結果 |
| --- | --- |
| 彩色 emoji | 過。Apple Color Emoji（sbix）經 swash 出 RGBA 點陣圖，👋 🙂 與原生一致 |
| 中日字形回退 | 過。locale `zh-CN` 下漢字全部落到 PingFang SC（含「日本語」「骨直曜」），沒拿 Hiragino 畫中文；假名落到 PingFang HK |
| 字型按需載入 | 過。5 個檔案 67 張面，release 1.5–4 ms（mmap，只解析名字表與 cmap）；預覽工具整程序峰值 RSS 16 MB |
| 文字觀感 | 過（mac）。補了三樣才對齊，見下；Windows 灰度抗鋸齒待 box 真機 |

首幀：2 倍屏 266×300 pt 一幀 release 1 ms（首幀 6 ms，含字形柵格快取冷啟動）。殼內字型庫 3.8 ms。

**對齊原生要補的三樣**（都是 CoreText 對系統字型默默做的事）：

1. **光學字號 `opsz`**：SF 是變數字型，CoreText 在 20 pt 以下用 opsz = 17（Text 視覺尺寸），cosmic-text 只設 `wght`，落在預設的 28（Display），
   小字號英文窄 16–24%。補法：給 cosmic-text 打補丁 `FontSystem::set_optical_size`（shaper 位置與 swash 柵格都帶 opsz），放在 [qingjian-team/cosmic-text](https://github.com/qingjian-team/cosmic-text) 的 `qingjian-opsz` 分支（基於 0.19.0，一個提交），workspace `[patch.crates-io]` 釘 rev；上游收了就回 crates.io。
   補丁是全域性一個值，字型例項快取沒按它分鍵；候選窗幾種字號都在 20 pt 以下落同一檔，夠用，做主題字號可調時要改成按字號分鍵。
2. **`trak` 字距表**：SF 按字號給每個字形加減間距（11 pt +12、12 pt 0、16 pt −40 個字型單位，值 / upem × 字號 = 點），PingFang 沒有正常軌。
   渲染器自己解析 `trak`（`fonts/trak.rs`），按字形所用字型各查各的。補完後「hello」「ni'hao」「1/6」「phr. you change」三個字號的寬度與 `NSAttributedString.size()` 到小數點後兩位相等。
3. **筆畫加深**：CoreText 對文字抗鋸齒有一層 gamma，線性混合出來的字偏細，深色背景尤其明顯。主題里加 `text_gamma`（淺色 0.85、深色 0.75），放大並排看筆畫粗細一致。

**剩下的已知差異**（每個字形 ≤ 1 pt，並排看不出，記著就行）：
- 「·」（U+00B7）CoreText 在中文系統上用 `.CJKSymbolsFallbackSC` 的寬點（5.76 pt），我們用 SF 自己的（3.56 pt）；一行譯文差 2 pt。
- 漢字 CoreText 用私有的 `.PingFang UI Text SC`（advance 0.993 em），我們用公開的 PingFang SC（1 em）。
- 假名 CoreText 用 `.CJKSymbolsFallbackSC`，我們用 PingFang HK。
- 雲朵是向量描邊近似 SF Symbol `cloud`，比原生略粗。

真機再過了一遍：糾錯後的拼音行（刪除線 + 淡色剩餘）、候選行裡的雲端詞、開著候選窗切系統深淺色、配置 `renderer` 熱切換兩個方向；1 倍外接屏沒裝置沒驗。
假名原先落到 PingFang HK，原因是 ヒラギノ角ゴシック W3 字重 300 被 cosmic-text 的字重匹配篩掉，換 W4 後落 Hiragino Sans。
筆畫加深用極性相關的覆蓋率 gamma 不是我們獨創：[muri #71](https://github.com/MattJackson/muri/issues/71) 得出同樣結論（macOS 的字型平滑按前景 / 背景極性調），
[skip.house](https://skip.house/blog/macos-font-rendering) 提到 Patrick Walton 逆向出了 macOS 的膨脹公式（pathfinder），以後想逐畫素對齊可以查它。

字型可選：`[general] font` 指定字族名，mac 殼用 CoreText 按字族名查出文件（`CTFontDescriptorCreateMatchingFontDescriptors` → `kCTFontURLAttribute`）交給渲染器只加載那幾個檔案，系統字型仍在後面當回退；沒裝就退回系統字型並記日誌。真機驗過 Kaiti SC 與不存在的字型名。

**結論**：四條都過，mac 上點陣圖渲染器可以替換 AppKit 繪製；Windows 半邊見下節。下一步做主題 TOML，穩定一版後刪 AppKit / GDI 舊路徑。
主題以後要放圖片 / 動圖 / 花邊：渲染器輸出就是一張點陣圖，裝飾只是多疊幾層，不用換底子。

## Windows 半邊（2026-09-15，真機已驗）

- **殼**：`apps/windows/server/src/ui/painter/`，候選視窗與懸浮狀態條共用一份渲染器（字型庫與字形快取一份）；`layered::present` 把渲染器出的預乘 RGBA 點陣圖換成 BGRA 後 `UpdateLayeredWindow` 貼上，陰影由渲染器畫（`Shadow::mac_panel()`，引數與 macOS 面板一致；分層視窗沒有系統陰影）。倍數取 DPI / 96。
  `[general] renderer = "system"` 時兩個視窗走原來的 GDI 畫法，與 macOS 一樣是過渡期退路；渲染器建不起來（字型庫載入失敗）也自動退回。
- **狀態條**：渲染器新增 `render_status`（`StatusCell::{Text, Gear}`，返回點陣圖與各格右邊界供點選命中）。齒輪改成向量畫：Segoe UI Emoji 排在回退鏈前面會把 U+2699 畫成彩色。
- **配置**：`[general] renderer` / `[general] font` 經 `CandidateSink::configure` 送到 UI 執行緒，裝上時與熱載入變了時各送一次；字族名按 DirectWrite 的系統字型集合找檔案（`qingjian_render::system_fonts`），設定程式的「字型」框也從它列字族。
- **候選行型別**：Windows 殼直接用渲染器的 `Row` / `Tone`，不再有自己的一份；macOS 殼還留著 `convert.rs`，spike 定型後一起去掉。
- **刪候選的提示**：渲染器畫在拼音行右側（與 macOS 一致）；GDI 畫法仍在拼音行下方。

真機結果（Windows 11 26200，2026-09-15）：候選視窗深色 / 淺色、豎排 / 橫排、Segoe UI Emoji（COLRv0）彩色、陰影，懸浮狀態條與向量齒輪，「渲染引擎 / 字型」設定項與熱切換（換成 Maple Mono NF CN 立即生效），`renderer = "system"` 退回 GDI，均通過。灰度抗鋸齒與微軟雅黑回退看著與 GDI 版沒有可感差異；Yu Gothic 回退與首幀耗時沒有單獨測。
排查中順帶發現並修掉的與渲染器無關的問題：TSF DLL 動態鏈 `vcruntime140.dll`，AppContainer 程序（工作列搜尋等）讀不到系統裡那份時整個 DLL 載入失敗、系統切回上一個輸入法，已改成靜態 CRT（倉庫根 `.cargo/config.toml`）。

## 不做的事

- 設定程式不自繪。三桌面端設定程式將來若要收攏，另議（Avalonia 是候選，Tauri 因 WebView 排除）。
- 手機鍵盤的殼仍是 Swift / Kotlin，只是面板內容由渲染器出點陣圖。
- 不引入 Flutter：多一門語言與工具鏈，收益只覆蓋設定頁，擴充套件支援未完成。

## 與既有計劃的關係

- 「Windows 候選窗 + 狀態條改 Direct2D + DirectWrite」已被本方向取代，作廢。
- `candidate-ui.md` 裡「每個平臺用原生繪製」與「Windows 用 Direct2D」兩句已按本文修正。
