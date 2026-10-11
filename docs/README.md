# docs

README.md 只介紹專案，所有技術內容放在這裡，分四類：前三類是給開發者看的內部文件，`user/` 是給使用者看的、官網渲染的文件。

| 目錄 | 放什麼 |
|---|---|
| [design/](design/) | 設計：現在是什麼樣、為什麼這麼定 |
| [plan/](plan/) | 計劃：接下來做什麼、按什麼順序 |
| [notes/](notes/) | 工程記錄：效能最佳化史、事故復盤、踩坑，按時間順序寫，以後複習與面試用 |
| [user/](user/) | 使用者文件：官網「文件」頁的內容源，按目錄結構渲染；維護約定見 [user/README.md](user/README.md) |

| 檔案 | 內容 |
|---|---|
| [contributing.md](contributing.md) | 開發約定：架構約束短版、程式碼組織、版本號、提交資訊、文件同步、提交前檢查、發版與外部 PR 流程（CLAUDE.md 直接載入它） |
| [design/architecture.md](design/architecture.md) | Core 與平臺層的劃分、crate 結構、必須遵守的架構約束、`.qj` 資料容器 |
| [design/candidate-ui.md](design/candidate-ui.md) | 候選視窗、按鍵約定與翻譯 annotation 的設計 |
| [design/landscape.md](design/landscape.md) | 同類項目（水杉、Rime）、可用資料來源及其許可 |
| [design/rendering.md](design/rendering.md) | 跨平臺 UI 與自繪渲染器（2026-09-13）：顯示面與控制元件面的區分、Flutter / Avalonia / WebView 的調研結論、主題的兩條路、tiny-skia + cosmic-text 的 spike 結果與兩平臺接入 |
| [design/aux-code.md](design/aux-code.md) | 輔碼（2026-09-14）：觸發鍵與過濾語義、Rime 碼錶匯入、筆畫原生表與大陸序覆蓋、候選註記與設定介面 |
| [plan/roadmap.md](plan/roadmap.md) | 分階段路線圖、各階段的依賴關係與已完成項 |
| [plan/zh_tw_support_plan.md](plan/zh_tw_support_plan.md) | 繁體輸出與台灣注音支援的分析與方案（貢獻者 pinchiu，#22）：讀音標準差異、台灣用語、簡轉繁一對多；兩條路線 |
| [plan/todo.md](plan/todo.md) | 待辦清單，按「從自用到能給別人用」排 |
| [plan/wubi.md](plan/wubi.md) | 五筆（形碼）支援方案（2026-09-15）：為什麼不能按雙拼的方式接、兩條平行管線、碼錶與詞頻、`[general] scheme` 收斂、分期 |
| [notes/crate-notes.md](notes/crate-notes.md) | 各 crate / app / tool 的實現要點：入口型別、資料檔案、常數、生成命令 |
| [notes/linux-fcitx5.md](notes/linux-fcitx5.md) | Linux 預設面板構建、協議、安裝與排錯 |
| [notes/performance.md](notes/performance.md) | 歷次效能最佳化：起因、定位方法、改法、數字前後對比與經驗 |
| [notes/release.md](notes/release.md) | 發版流程：CHANGELOG、標籤觸發的 CI 打包、產品資料包、簽名公證的 Secrets、官網用的 `releases.json` |
| [notes/model-identities.md](notes/model-identities.md) | 含章·通變與含章·知微的正式名稱和檢查點編號 |
| [notes/phrase-layer.md](notes/phrase-layer.md) | 短語層（2026-09-12）：常用詞表收不到的 我的 / 不知道 怎麼從語料挖、怎麼進語言模型而不傷整句、兩把尺子的前後數字 |
| [notes/domain-words.md](notes/domain-words.md) | 領域詞（2026-09-12）：從輸入日誌人工挑 48 條進基礎詞庫的挑法、低頻詞當 token 統計為什麼傷整句、兩把尺子的前後數字 |
| [notes/constant-sweep.md](notes/constant-sweep.md) | 排序常數掃描（2026-09-12）：插值與敲錯代價在凍結日誌上掃網格，全在平臺區不改；沒命中的構成與復現步驟 |
| [notes/windows-win10.md](notes/windows-win10.md) | Windows 10 與設定程式（2026-09-13）：Reactor 早期繫結 Windows 11 才有的 AppModel API 導致載入期失敗，改自包含部署 + 延遲載入 |

約定：文件寫中文，程式碼識別符號一律英文。實現與文件產生分歧時以程式碼為準，並同步更新文件。
