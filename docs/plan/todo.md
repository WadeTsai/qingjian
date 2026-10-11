# TODO

按「從自用到能給別人用」排。勾掉的移到 [roadmap.md](roadmap.md) 對應階段。`★` 是當前建議的優先項。
2026-09-06 狀態：核心輸入、翻譯（中→英 / 日 24.9 萬詞、英→中 4.5 萬詞、注假名、修飾鍵上屏譯詞、翻譯選中文字）、英文模式候選、
自建詞庫（基礎 8.7 萬條 + 隨包領域詞庫 11 本 13 萬條，含語料挖出的高頻詞）、多詞庫與匯入 / 移除 / 開關、偏好設定、`.qj` 容器（啟動 50 ms）、
個人 n-gram（二元 + 三元）、詞圖內敲錯邊、崩潰不丟、pkg 安裝器、README 安裝 / 隱私 / 許可都已完成並在自用。
給測試者打包只剩：特殊應用驗證（使用者自己在各應用裡試）；簽名等 Developer ID 證書。

## 〇、上線前必須做（不做就不能給別人）

- [ ] 詞庫質量（產品詞庫已自建，見 roadmap）：常用詞表只有 5.6 萬，語料挖出的 1.57 萬高頻詞已併入（蚌埠 / 台州 / 高鐵 / 吐槽 這類補上了），
  網路用語目錄還空著（`assets/lexicon/04_internet_slang`）；LLM 標註的多音字讀音抽查；挖詞過濾已進 `dict-convert mine`（2026-09-07，`oov_filter.rs`，與原指令碼輸出逐行一致）；
  LM 表（TSV 48 MB / `lm.qj` 29 MB）不進 git，先按釋出附件對待
- [ ] ★ 首次執行體驗：沒有 `data/generated/` 時打進包裡的樣例詞庫只有幾十條，等於不能用；隨包資料（詞庫、LM 表、釋義表、emoji）
  要有一套釋出版並定好體積（現在 `dict.qj` 3 MB + 領域詞庫 7 MB + `lm.qj` 29 MB + 釋義表 `.qj` 39 MB + 英文詞表 2 MB，pkg 34 MB；可再考慮變長編碼）；
  釋出物 = pkg（`bundle.sh --pkg`），要麼 CI 生成資料、要麼把 `data/generated/` 當釋出附件
- [ ] 倉庫瘦身（2026-09-07 決定：主倉庫繼續單庫、官網獨立倉庫，不拆程式碼；要處理的是資料）。
  已做：LLM 中間產物 `gloss-llm.jsonl`（30 MB）/ `gloss-en-llm.jsonl` / `pinyin-llm.jsonl` 移到 `data/generated/`（gitignore），倉庫只留匯出的 `glossary-*.tsv` 與詞庫源；
  歷史在同日壓成 `v0.1.0` 單提交（完整開發歷史在本地分支 `archive/dev-history`，不推），所以公開倉庫裡從沒出現過這些檔案。
  還差：倉庫推到 GitHub 後跑一次 `tools/release/data-bundle.sh`，把這三個 JSONL 與 `data/generated/*.qj` 傳到 `data` 預釋出 Release（現在只在本機與歸檔分支裡有）；
  正式開源後若 clone 太慢，再考慮把 `assets/lexicon` + `assets/glossary` 拆成資料倉儲，現在不做
- [ ] ★ 簽名與公證：pkg 安裝器、自動註冊輸入源、版本號 / 構建號已做（2026-09-06，`bundle.sh --pkg`）；
  CI 已搭（2026-09-07，`docs/notes/release.md`）：推標籤在 macOS runner 打 arm64 + x86_64 兩個 pkg、建 Release、生成 `releases.json`，產品資料從 `data` 預釋出 Release 下載。
  還差：Apple Developer 賬號的 Developer ID 證書（配齊 `release.yml` 頭部的七個 Secrets 就自動簽名公證）、倉庫推上 GitHub 後第一次真跑驗證（本機沒跑過 workflow）、
  官網改成讀 `releases.json`（含 sha256 / 提交 / 構建時間的展示）並配 `QINGJIAN_WEB_TOKEN` 讓發版與文件改動自動觸發官網構建、更新檢查
- [ ] ★ 特殊應用逐個驗證（2026-09-07 使用者已驗：Zed、Terminal 正常；JetBrains 系沒裝沒驗）：Secure Input（已有檢測，驗證密碼框不組句、不發雲端）、iTerm / Warp、VS Code / Electron、
  瀏覽器位址列、沙盒應用（App Store 版備忘錄 / 微信）、全屏遊戲；每個記錄 preedit 模式建議（按應用的開關已有 `[apps]` 分節，
  英文候選按應用關已做，preedit 模式按應用定可以接在同一分節）
- [ ] 登入後不恢復成青簡：2026-09-06 兩次開機日誌都顯示登入後當前輸入源是系統拼音 / ABC，要手動切回（程序沒崩過，panic 與崩潰報告都是零；
  Caps Lock 切 ABC 的系統開關已排除）。懷疑裝在 `~/Library/Input Methods` 的 ad-hoc 包每次重灌簽名變、登入早期列舉沒認上；等簽名後裝 `/Library` 再看，
  或在 `--register` 之外登入時補一次 `TISSelectInputSource`
- [ ] 解除安裝與重置：解除安裝指令碼已隨包（`uninstall.sh`，`--purge` 連資料刪，README 有說明，2026-09-06）；還差偏好設定裡的「重置學習資料」按鈕、
  配置檔案版本遷移（新增鍵有預設，改名鍵要遷）
- [x] 許可證定稿（2026-09-07）：GPL-3.0-or-later，`LICENSE` 全文、`Cargo.toml` `license`、「關於」頁、README 一致；名字與 logo 不授權。以後引入 JMdict 要補進署名清單


## 一、日常輸入體驗（決定自己會不會一直用）

- [ ] 整句轉換收尾：語料再補（現在是維基 + LCCC 微博對話，新聞 / 小說體還缺；bigram 只留次數 ≥ 6 的前 300 萬對，
  `wxqcf` 出 我想去蹭飯 就是「我想 → 去蹭飯」8 次進了表、「我想去 → 吃飯」沒進的稀疏問題，簡拼把讀音放開後這類問題更顯眼）；
  個人 n-gram 已做（二元 + 三元），「按應用分別記」看用起來的效果再說
- [ ] 釋義兜底收尾：已做（2026-09-06，只對上屏的詞庫詞 / 雲端詞查，個人釋義表 `user-glossary-<語言>.tsv`）；還可以考慮停頓後高亮的候選也查、
  偏好設定裡看 / 清個人釋義表、匯入詞庫時批次預填
- [ ] 候選窗口裡超長釋義要截斷（CEDICT 表時 你 那條能拉到整屏寬；LLM 表已短，仍要兜底）
- [ ] 翻譯選中文字：譯文很長時候選視窗的折行（讀不到選區的提示已做）
- [ ] 英文模式候選：把資料包的誤拼對照表（typos，10 萬對）喂進糾正，技術詞（kubectl）沒有 wordfreq 詞頻要給底值，兩處編輯的糾正
- [ ] 全形 / 半形切換，`。` 與 `．`（`-` `=` 非組句時的行為 2026-09-15 已定：仍是半形，但改由殼自己插入，
  不再靠放行——放行在部分宿主裡到不了應用，見 `candidate-ui.md` 的標點一節）
- [ ] 快捷短語（`i` 字首）：編碼 → 短語表，使用者可增刪
- [ ] 問字模式離線版：拆字表（IDS 資料）按部件查字，雲端版已有
- [ ] 雙拼收尾：選單欄增加方案切換
- [ ] 雙拼方案資料化：加方案不用改程式碼（放一張鍵位表就能用，與 #51 的方案匯入一起定）；解碼支援同碼出多個讀音——小浪的 `lk`（lai / lia）、`nm`（niang / nen）、`dk`（dai / dia）、`un`（en / eng）現在只解第一個，做完去掉 `every_syllable_round_trips` 裡小浪的四個例外（#129）
- [ ] 輔碼（issue #8，設計見 [docs/design/aux-code.md](../design/aux-code.md)）：觸發鍵（預設 `;`，與「非拼音鍵進直輸段 / 全形標點」排優先順序）與逐鍵即篩、詞級過濾、輔碼態隱藏無碼詞；碼錶走 Rime yaml 匯入（`columns` 必須解析、`import_tables` 跨檔案合表、「有詞無碼」行要報警）；碼錶存**獨立碼索引**（兩段式查詢，別複用詞庫 Slot）；
  原生表只帶筆畫（CNS11643 筆順 + 大陸序覆蓋表 `dict-convert stroke`，艹 3 / 辶 3 / 阝 2）；設定介面加「輔碼」頁（macOS 偏好設定 / Windows 設定程式：觸發鍵、顯示碼、碼錶列表、匯入）；第三方形碼錶（小鶴形 / 自然碼形）不隨包，走引導匯入
- [ ] 敲錯邊收尾：敲錯四類代價與整段糾錯代價 2026-09-12 已在凍結日誌（12886 條可評）上掃過，都在峰上，不改（`docs/notes/constant-sweep.md`）；
  個人折扣上限（3）回放從零學分不出好壞，等 `user-typos.tsv` 能帶進回放（`--user-dict`）再看；
  退格重打（組句內、跨上屏）已進輸入日誌 `retype` 事件（2026-09-12），攢夠後先按 `docs/plan/model-eval.md` 的尺子算召回與誤糾率，再決定喂不喂個人敲錯表；
  整段一處編輯那路仍是逐變體跑整句，長句慢（`nihoama` 1 ms、更長的幾毫秒）
- [x] 個人 n-gram 插值常數（λ 0.8 / K 8 / 上限 0.5 / D 0.75）2026-09-12 回放掃過：K 全平，λ 0.7–0.75 與 0.8 差 0.3 個點、帶神經重排後無差，封頂 0.7 起掉；保留現值（`docs/notes/constant-sweep.md`）。
  「擠掉了」要選四次才翻過「給掉了」是 K 與封頂合起來的設計（一次誤選翻不過），不是常數沒調
- [x] 詞庫缺高頻「代詞 + 的」（我的 / 你的 / 他的）：2026-09-12 加了短語層 `assets/lexicon/phrases.tsv`（`dict-convert phrases`，語料相鄰兩三詞、對話語料裡 ≥ 2000 次、
  邊界規則，5400 條：我的 / 好的 / 不知道 / 有沒有 / 那我），品牌詞 `brand.tsv`（青簡 210）；詞典詞頭收不到的這一層以後按同一方法補
- [x] 語料裡少的領域詞（對齊 / 後端 / 詞庫 / 候選框）：2026-09-12 從日誌人工挑 48 條進 `assets/lexicon/domain_words.tsv`，語言模型走合成計數（`docs/notes/domain-words.md`）；
  語料 0 次的（微軟拼音 / 懸浮條）按規矩沒進，要進得另立白名單
- [x] 中英混雜詞與英文專名（2026-09-15）：`assets/lexicon/mixed_words.tsv`（C盤 / B站 / 隨身碟 / T恤 / A股…，`cpan`→C盤，`youpan`→隨身碟，
  可進整句 `wozaibzhan`→我在B站）；`dict-convert english` 同編碼優先大寫專名（Windows ≠ windows），`07_display_forms.tsv` 補 VSCode / Bilibili 等；
  產品 `dict.tsv` / `english.tsv` 已更新。句中英文仍只支援句末尾段，見「中英混輸」。
- [ ] 按輸入串記的選擇只認字面：`wod` 下選的 我的 惠及不到 `wode`；考慮同時按候選全拼記一份、查詢取兩者最大
- [ ] 已經學進使用者詞的錯讀音雲端詞（`我的 wo di`、`我的哦 wo di e` 這類）沒有清理入口：偏好設定詞庫頁給「按讀音核對使用者詞」，或一次性指令碼
- [ ] 正式版前的釋出可信性（2026-09-12 外部 CI 檢查，測試版先不做）：產品資料改不可變 tag 並在倉庫鎖版本 + SHA（2026-09-16 已做：`data-vN` Release + `tools/release/data.lock` + `data-fetch.sh`）；
  安裝包內容驗證（pkg / Setup.exe 裡詞庫、模型、許可齊不齊，`codesign --verify` / `signtool verify`）；`cargo deny`（許可證 + 來源）；`.qj` 讀取器越界 fuzz、`qingjian-format` 跑 Miri
- [ ] 本地整句模型（已進殼並隨包發出，見 `docs/notes/neural-rescoring.md`；載入 12 秒是早期首次 Metal 編譯的記錄，2026-09-12 裝機實測 102 ms，劃掉）：
  重排改了切分時應用裡的行內拼音要到下一鍵才更新；日語
- [ ] 個人微調（LoRA 掛在凍結基模上，基模更新後拿本地日誌重訓）：先在 train 倉庫驗證收益，再驗 candle CPU 訓練能不能跑通，都過了才進殼
  （閒時門禁、載入時合併權重、開關與一鍵清除）；每個模型假設先按 `docs/plan/model-eval.md` 寫尺子與基線
- [ ] 個人英文詞表沒有衰減（2026-09-24 發現）：`learn_english` 只加不減，`insert_english` 裡的 `choice_weight` 只決定
  「要不要當第一」（`english_first`），位置只有 0 和 1 兩檔——使用者選一百次中文候選，那條英文還是釘在詞級候選第一位。
  於是打不出來時原樣上屏攢下的失敗拼音（`houxuanshengcheng`、`shururizhi`）會永久佔第一頁一格，不危險但也不自愈。
  合理的改法是讓選擇次數參與位置而不只是決定第一名，屬於排序改動，先跑 `--replay` 凍結日誌與整句評測再動。
  現成出口：刪候選快捷鍵（預設 `Shift` + 數字）走的就是 `forget_english`。
- [ ] 日語 emoji 要等有日語輸入模式

## 二、產品特色

- [ ] 釋義表收尾：覆蓋面再擴（次數閾值降到 100 約 10 萬詞）；JMdict 校對假名與日文詞性；等級表（CEFR / JLPT 已隨包，見 `assets/levels/`）
  也可做英→中、日→中方向的種子；四六級 / 商務英語詞表要先找到許可乾淨的來源
- [ ] 可選複習（Phase 4）：輸入統計、生詞識別、詞彙統計（含 CEFR / JLPT 等級分佈）已做（2026-09-06）；複習容易變成打擾，先不急
- [ ] 雲聯想收尾（Phase 6）：金鑰進鑰匙串（等簽名定了再做，ad-hoc 簽名每次重灌都彈授權）、按應用停用、限流與用量統計
- [ ] 個人模型（Phase 7）：小 Transformer 實驗（有評測門檻），見 roadmap

- [ ] ★ **主題與自繪渲染器 spike**（2026-09-13 定向，分支 `renderer-spike`，見 [design/rendering.md](../design/rendering.md)）：
  tiny-skia + cosmic-text 畫一行「青簡 hello 🙂 日本語」+ 圓角陰影，Windows / macOS 與原生並排截圖，驗四條：彩色 emoji（sbix / COLRv0）、
  中日字形回退按 locale、字型按需載入（不掃全系統）、灰度抗鋸齒觀感；首幀耗時與記憶體不劣於 GDI / AppKit。過了 Windows + macOS 一起換渲染器、主題檔案 TOML；
  不過退回各平臺各自渲染（Windows 走 D2D）。設定程式不自繪。

## 三、其他平臺

- [~] Windows TSF（Phase 5）：真機自用中，候選窗已覆蓋商店 / 工作列搜尋（`uiAccess` + 自籤）。
  與 mac 功能齊平尚缺（按價值排；工時=開發+真機驗證合計，單人；真機來回是大頭）：
  - [~] **① 翻譯選中文字 `translate_selection`**（★★★ / 中高 / 1.5–2 天）：**程式碼完成，待真機測**（2026-09-10）。
    新增 `RequestSelection` / `Selection{text,rect}` 兩協議訊息；DLL 非同步只讀會話（`com/selection.rs`，`GetSelection`+`GetText`+`GetTextExt`）
    讀選區 → Server `request_translation`（`PredictionKind::Translate`）→ 自繪候選窗顯示譯文 → 回車 / 空格替換（回 `commit` 走 `InsertTextAtSelection` 替換選區）/ Esc 保留；
    設定頁「快捷鍵」加了翻譯選中文字行（改修飾鍵）。mac 全綠 + windows-gnu 交叉編譯 / clippy 過；沉浸式應用讀選區、替換選區待真機驗。
  - [~] **② 生詞「看到輪次」計數 `note_displayed`**（★★☆ / 低 / 0.5 天）：**程式碼完成，待真機測**（2026-09-10）。
    `dispatch/mod.rs::reconcile_candidates` 裡真正 show 那次按當前頁調 `Engine::note_displayed`、收窗傳空，對齊 macOS `render`；輪次推進、生詞滿 3 輪畢業。
  - [~] **③ 懸浮狀態條**（★★☆ / 中 / 1.5–2 天）：**程式碼完成，待真機測**（2026-09-10）。
    Server 側單例，跟候選窗同一條 UI 執行緒；分層窗合成器從候選窗 `surface.rs` 抽出成 `ui/layered.rs` 兩邊共用，繪製件（主題 / DPI / 深淺）複用；
    狀態條自己一個視窗類 + 視窗過程（`ui/status/`）：`WM_NCHITTEST`→`HTCAPTION` 整塊可拖、`WM_MOUSEACTIVATE`→`MA_NOACTIVATE` 不搶焦點、
    `WM_EXITSIZEMOVE` 把位置寫回 `[status_bar] x/y`（`GWLP_USERDATA` 存自身指標）。新增協議 `ClientMessage::ModeChanged{english}`（fire-and-forget），
    DLL 在 `update_mode_indicator` 推模式；Server 據此刷、會話關就收起；雙拼方案 Server 從 `[general] shuangpin` 知道。配置加 `[status_bar]` 分節（`enabled`/`x`/`y`），
    設定「候選視窗」頁加開關。參考水杉 FTB 形態（它用 D2D 且不記位置），我們沿用 GDI 分層窗保持視覺一致並加了記位置。
    mac 全綠（+3 狀態條分派測試）+ windows-gnu 交叉編譯 / clippy 過；拖動 / 記位置 / DPI / 深色 / 蓋高 z-band 待真機驗。
  - [~] **④ 刪候選螢幕提示**（★☆☆ / 低 / 0.5 天）：**程式碼完成，待真機測**（2026-09-11）。
    `Frame` 加 `notice: Option<String>`（不參與 `is_empty`），`dispatch/shortcut.rs::forget_on_page` 填、`handle_key` 開頭清（只活到下一次按鍵）；
    自繪候選窗 `ui/candidates/view.rs` 畫在拼音行下方（淡色小字）。host 測已加。
  - [~] **⑤ 工作列點中/英反同步**（★☆☆ / 低 / 0.5 天）：**程式碼完成，待真機測**（2026-09-11）。
    啟用時對轉換模式 compartment 掛 `ITfCompartmentEventSink`（`com/conversion.rs`），`OnChange` 讀回 `NATIVE` 位、與當前模式不同才翻轉
    （防迴環），順帶刷指示器 + 上報 Server 讓懸浮狀態條也同步。純 DLL 改動、無新協議。
  - [ ] 發版：換 **Certum 開原始碼簽名證書**重籤（開發全程自籤 + 本機受信任根，見 `installer/sign-local.ps1`）、
    `windows-v<版本>` 標籤與 CI。
  - [ ] **本地整句模型上 Windows**：Server 已接（`dispatch/rescore/`，CPU 推理，設定「雲服務」頁有開關，安裝包帶 `data\model`），待真機驗：每次重排的耗時（前文 + 幾條路徑一次前向，CPU 上可能幾十到一百多毫秒，超了就縮前文長度）、模型載入時間；
    應用游標前文已接（DLL 起組句時讀、密碼框跳過；2026-09-12 真機驗過記事本 / Edge / 終端都讀得到，Edge 密碼框按 `IS_PRIVATE` 識別），真機看沉浸式應用讀不讀得到；
    模型單檔案 `.qjm` 已做（2026-09-12，複用 `.qj` 容器 `Kind::Model`，`find_model` 先 `.qjm` 再三件套目錄，`pack model` / `tools/release/pack-model.sh`，
    data Release 傳 `model.qjm`，bundle.sh / qingjian.iss 只帶一個檔案），待 mac 與 box 真機各裝一次驗載入與重排；
    密碼框已按 TSF 規範做（2026-09-12）：`KEYBOARD_DISABLED` compartment 整鍵放行不組句，`IS_PRIVATE` / 密碼 / PIN 輸入範圍為私密（組句但不學不記不發雲端，`ClientMessage::Privacy` → `Engine::set_private`），box 真機驗過：Edge 密碼框整鍵放行；InPrivate 網頁文本框報 `IS_SEARCH` 不報 `IS_PRIVATE`，私密路徑只靠單測覆蓋；CI 兩個 job 都從 `data` Release 取 `model.qjm`（已做）。
- [ ] Linux Fcitx5 後續（預設面板與手動安裝已實現）：native Wayland 驗證、Server 自繪 / GNOME 點陣圖、神經重排、自動啟動與打包；配置同步、跨平臺詞庫

## 四、其他輸入方案

- [ ] 五筆（86 版）：方案與分期見 [wubi.md](wubi.md)，第一期 Core + CLI + Windows；
  `[general] scheme` 收斂與配置遷移一起做（順帶解掉〇裡的「配置檔案版本遷移」）
- [ ] 複雜方案收尾（與上一條共用「輸入方案」抽象）：注音只在 Core 與 Windows 接了，macOS 側還沒接；雙拼的方案切換要等 `[general] scheme`
