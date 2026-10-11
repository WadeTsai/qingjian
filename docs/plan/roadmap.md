# Roadmap

各階段有依賴關係：先把 Core 和 CLI 打通再碰平臺 API，否則會在沒有可測試核心的情況下除錯 IMK。

## Phase 1 — Core

- [x] 基礎 Composition 狀態機
- [x] Candidate 資料模型（含詞性 + 譯文的 annotation 結構）
- [x] 拼音解析（全拼、`'` 分隔、末尾殘缺音節）
- [x] ü 韻母規範化：全拼 `lue` / `nue` 與詞庫 `lve` / `nve` 等價，生成與匯入統一使用 `v`
- [x] 中文候選生成（詞級）
- [x] 基礎詞庫（TSV 記憶體詞庫，`assets/sample/` 為手寫樣例）
- [x] 候選排序（詞級規則排序）
- [x] CLI 測試工具（`apps/cli`：候選 + 詞性譯文 + 各階段耗時，序號上屏記入使用者詞頻）
- [x] 音節表查詢改為 trie
- [x] 簡拼（聲母縮寫 `kf` → 開發，與全拼混用，marked text 自動補 `'`）
- [x] 開發測試詞庫：曾用霧凇拼音（GPL，不可釋出）；2026-09-05 起換成自建詞庫，見 Phase 2 末尾「產品詞庫」
- [x] 中→英釋義：`tools/dict-convert` 從 CC-CEDICT 生成 11.5 萬條（CC BY-SA 4.0，可釋出）
- [x] 帶詞性的中英 / 中日釋義表：`tools/gloss-gen` 用 LLM 批次生成（2026-09-05 起覆蓋整個自建詞庫 23.9 萬詞，詞性 + 英文譯詞 + 日文譯詞與假名，`assets/glossary/`），
  `Sense.reading` + 漢字注平假名（`candidate::furigana`）；取代 CC-CEDICT 表做隨包資料
- [x] Option+數字上屏候選的譯文（`Engine::commit_translation`，控制器在 `handleEvent:client:` 裡按鍵碼認，其餘交父類）
- [x] 整段切不動時取能切分的最長前綴出候選，剩餘字母作為未切分尾部保留（`kaifv` → `kai'f'v`）
- [x] 整句轉換（bigram + Viterbi），含簡拼整句（`wxqcf` → 我想去蹭飯 / `jttqhh` → 今天天氣很好）
- [x] 模糊音（Core `fuzzy`，配置 `[fuzzy]`）：z/zh、c/ch、s/sh、n/l、f/h、l/r、an/ang、en/eng、in/ing，
  詞庫按「每個位置多種寫法」一次查出，模糊命中詞頻減半排序，整句轉換同樣生效
- [x] 偏好設定「關於」頁（2026-09-05）：版本 + 構建號、測試版許可（倉庫 `LICENSE` 由 MIT 改為測試版保留所有權利，開源時再定）、資料署名、隱私說明、
  「開啟日誌目錄」「複製診斷資訊」；日誌預設 info、`[general] log_level` 熱切換
- [x] 雙拼（2026-09-05，Core `shuangpin`）：小鶴 / 自然碼 / 微軟 / 搜狗四套方案，鍵 → 全拼解碼後複用全拼的切分與整句，配置 `[general] shuangpin`，
  偏好設定「通用」頁彈出選單，CLI `--shuangpin`
- [x] 查詞效能：`lookup_pattern` / `lookup_exact` 逐級字首二分收窄（簡拼位置按音節塊跳掃，小區間線性），
  同一次查詢內字首模式記憶化，排序鍵預計算 + 只選前 500 條。長輸入 20 ms → 0.2 ms，全簡拼 `zhgdoima` 70 ms → 1 到 3 ms，單字母 30 ms → 3 到 4 ms
- [x] 逐鍵效能（2026-09-05，`qingjian-cli --typing` 按字首逐鍵計時）：糾錯變體先用無分配的「能否完整切分」過濾
  （`zhzhzh…` 16 鍵 parse 39 ms → 0.2 ms）；整句格子候選跨按鍵快取（`sentence::SpanCache`，上屏 / 學習時清，
  `sss…` 15 鍵 rank 12 ms → 1 ms）；bigram 表改成按前詞分組的 CSR 佈局；詞頻 / 語言模型 / 個人 n-gram 的雜湊表換 foldhash；
  預選鍵打包成 u128；沒開模糊音不逐條對模糊命中。結果：全拼、簡拼長句每鍵 0.3 到 3 ms，單字母首鍵 5 ms（模糊音全開 8 ms），
  英文模式每鍵 0.3 ms 以內；啟動 0.9 s（詞庫 0.24 s + LM 表 0.62 s）
- [x] 資料二進位制化（2026-09-05，`crates/qingjian-format`）：`.qj` 容器 = 頭 + 分節表 + `META`（名稱 / 許可證 / 署名）+ 原樣落盤的記憶體陣列，
  mmap 零複製開啟；詞庫與語言模型都進了容器（`dict-convert pack dict|lm`），啟動 0.93 s → 0.05 s（詞庫 12 ms + LM 9 ms）
- [x] 產品詞庫（2026-09-05，見 Phase 2 末尾）

以上「已完成」指 2026-09-06 的狀態：自用能用，詞庫已是自建的產品資料（基礎 + 11 本隨包領域詞庫），學習資料原子寫、壞檔案容忍（見 Phase 2「崩潰不丟」）。

## Phase 2 — macOS

- [x] IMK 空殼（打包、安裝、註冊、按鍵到達鏈路）
- [x] Input Method Kit 接入 Core（`apps/macos/src/host.rs` 程序級單例 Engine）
- [x] 輸入事件處理（字母 / `'` / 數字選詞 / 空格 / 回車 / 退格 / Delete / Esc / 上下鍵選詞 / 左右鍵與 Cmd+左右移動拼音游標）
- [x] Composition 顯示（marked text 顯示原始拼音）
- [x] 自定義豎排候選視窗（`candidate_window.rs` + `candidate_view.rs` 自繪，`theme.rs` 集中可視引數）
- [x] 中文輸入
- [x] 候選選擇與上屏（上屏後剩餘拼音繼續組句）
- [x] 候選翻頁（PageUp、PageDown / Tab、Shift+Tab / `[` `]`（預設）或 `,` `.`（配置）翻頁，上下鍵到頁邊自動翻頁，右下角頁碼；組句期間未識別的編輯動作一律吞掉）
- [x] 拼音游標編輯：左右鍵移動游標，插入 / 退格 / Delete 相對游標；marked text 裡顯示真實游標
- [x] 游標作用域：游標停在中間時候選、雲端詞校驗、聯想請求都只看游標前的拼音（`Composition::scope`），上屏吃掉作用域後游標落到末尾
- [x] 整句轉換（Core `sentence`）：詞圖 + Viterbi + 束搜尋，`Dictionary::lookup_exact` 精確長度查詞圖格子；整句排第一，`CandidateKind::Sentence`
- [x] 簡拼整句：簡拼位置按字首取詞進格子（格子多留到 20 個），語言模型在路徑上挑讀音；`jttqhh` → 今天天氣很好、`wjdzjsg` → 我覺得自己是個；
  兩字母簡拼仍是詞優先（`sj` 時間 / `zg` 這個，Viterbi 單詞路徑分高就不出句子）；有音節沒轉成字的不算句子；英文詞 / 英文補全留在句子前面
- [x] bigram 語言模型（`qingjian-lm`，`LanguageModel` trait 注入）：`dict-convert bigram` 從中文維基 + LCCC 統計，`pack lm` 打成 `lm.qj` 隨 bundle 打包，沒有時退化為一元
- [x] 個人 n-gram（Core `sentence::UserNgram`，`Learner::user_ngram()`）：上屏詞序列線上計數，與靜態模型插值進 Viterbi；`user-ngram.tsv` 落盤；
  2026-09-06 加三元（絕對摺扣回退到二元，Viterbi 前二詞取前驅回指，`CommitChain` 記兩個詞）
- [x] 自動造詞：連著選出的兩個詞合起來詞庫沒有、記夠次數就成使用者詞（同段拼音兩次 / 分段三次）
- [x] 快捷候選（Core `shortcut`）：`rq` / `sj` / `xq` 出日期 / 時間 / 星期，`v` 開頭表示式模式出四則運算結果與中文數字（見 candidate-ui.md）
- [x] 中文標點（Core `punctuation`：全形對映、引號配對、數字後的點保持半形；組句期間 `,` `.` 把高亮候選上屏再補標點；配成翻頁鍵時只翻頁）
- [x] 候選框頂部自繪 preedit 行與游標（不依賴應用畫插入點）
- [x] bundle id 改為 `app.qingjian.inputmethod`（2026-09-06，域名 qingjian.app 註冊後；`TISInputSourceID` 與連線名同步，舊 id `com.yenharvey.qingjian` 的輸入源要在系統設定裡刪掉重加）
- [x] 中英切換：Caps Lock 亮著 = 英文模式（預設小寫、Shift 大寫、標點半形）；按住 Shift 的大寫字母直接透傳；切換輸入源時強制收窗
- [x] 英文模式候選（Core `english`，`Engine::set_english_mode`）：詞表精確詞 / 字首補全 / 一處編輯糾正，Tab 與上下鍵選詞（上下鍵動過之後空格也選），空格回車標點原樣上屏；
  `[general] english_candidates` 可關；CLI `--english-mode`
- [x] 按應用關英文候選（2026-09-05）：`[apps] english_candidates_off`，按 `bundleIdentifier` 認，預設終端 / 編輯器 / IDE 名單，`*` 字首匹配；
  偏好設定「通用」頁勾選框；`[apps]` 分節留給以後的按應用 preedit 模式。2026-10-10 改成白名單 `english_candidates_on`（預設瀏覽器 / 聊天 / 辦公），
  Photoshop 這類靠單鍵快捷鍵的軟體列不完
- [x] 個人英文詞表（2026-09-05，`user-english.tsv`）：回車 / 英文模式直通原樣上屏的英文詞（切不成完整拼音的字母串）與選過的英文候選都記，
  與隨包詞表一起出英文候選且在前；修的是 `gist` 這類隨包詞表裡沒有的詞永遠出不了候選、回車多少次也學不會的問題
- [x] 中英混輸：整段輸入在英文詞表裡即出英文候選，作為拼音「不像話」時排第一，否則排第二（詞表 2026-09-05 起來自 `assets/lexicon/05_english`，ESDB / CSpell，MIT）
- [x] 英文補全：拼音不像話且 ≥ 3 個字母時出該字首下最常用的 3 個英文詞（詞頻 wordfreq），第一個字母就切不動的輸入也出
- [x] 英文直輸段：組句中敲 `-` 後整段原樣上屏（`no-way`），`-` 不再翻頁
- [x] emoji 候選（Core `emoji`，Unicode CLDR 中文 annotations，`assets/emoji/`）：緊跟對應詞，右側標註詞
- [x] 金鑰：輸入法程序讀配置同目錄的 `.env`（launchd 看不到 shell 環境變數）
- [x] 學習語言、每頁候選數、翻頁鍵、外觀、模式鍵從配置檔案讀取（`[general]` / `[shortcut]`），儲存後自動熱載入
- [x] 應用圖示與輸入法選單圖示（`assets/icon/logo.png` → bundle.sh 生成 icns；選單圖示 `assets/icon/menu.pdf` 模板圖隨深淺色反色）
- [x] 選單欄「中 / 英」狀態項（NSStatusItem，啟用時顯示，定時輪詢 Caps Lock）
- [x] 輸入法選單（狀態項 + 系統輸入源選單共用一份 NSMenu：雲聯想 / 模糊音勾選、偏好設定、日誌目錄、版本）
- [x] 退格撤銷學習：上屏後整個退格刪掉再重打同一段拼音換選，上一次的學習退回去
- [x] 輸入日誌（2026-09-05）：`input-log.jsonl` 每次上屏一行（鍵 / 切分 / 前 5 候選 / 選了第幾個 / 來源 / 糾錯 / 撤銷 / 時間），只寫本機，
  `[general] input_log` 預設開，「進階」頁開關 + 清空；Core `InputLogger` trait，學習 crate `InputLog` 落盤；離線回放評測與個人模型的資料來源
- [x] 回放評測（2026-09-05）：CLI `--replay input-log.jsonl`，按來源算首選 / 前五命中率、平均名次，列印沒命中的例子；記憶體學習不寫檔案
- [x] 輸入日誌 v1（2026-09-12，`docs/plan/model-eval.md`）：日誌當評測基礎設施，每個模型假設先有尺子再立項。新增事件 `session` / `retype`（組句內退格重打、
  跨上屏相近重打）/ `passthrough` / `prediction` / `break`，上屏行加 `rescored` / `app` / `pages` / `ms`、候選記一頁 9 條；空的原樣上屏不再記
  （舊日誌一半是它）；回放報告加重打、聯想接受率、隱式訊號（命中 / 未命中兩組的翻頁與耗時）、重排 / 未重排兩組命中
- [x] 整句評測集（2026-09-08）：CLI `--eval-text <中文文本>...`，使用者自己的文本切句、按詞庫讀音轉全拼、冷啟動還原，`--eval-save` 凍結句子集；
  基線（本倉庫 docs + 個人技術筆記 8322 句）：首選 37.6%、字準確率 76.9%，技術文本里的行話（候選 / 詞庫 / 釋義表）是主要丟分處
- [x] 刪除候選（2026-09-05）：⇧ + 數字（`[shortcut] delete_candidate`），使用者詞整個刪、詞庫詞清全部學習（`Learner::forget`，個人 n-gram `forget_word`），拼音行右側顯示結果
- [x] 拼寫糾錯：一處編輯（換位 / 換字母 / 多一個 / 少一個）湊出完整音節，拼音行畫刪除線，回車原樣上屏，接受與拒絕都進學習；
  2026-09-05 挑選改成噪聲通道（糾正後整句得分扣編輯代價仍高於原樣才糾，末尾單字母只試換位），修掉 你後媽 / 我峽谷區 這類選錯
- [x] 詞圖內敲錯邊 + 個人敲錯表（2026-09-06）：整句詞圖裡每個完整音節按一處敲錯變體（相鄰鍵換位 / 相鄰鍵 / 多鍵 / 少鍵）查詞並扣代價，
  `meiganxi` → 沒關係；接受的 (敲的, 要的) 音節對記 `user-typos.tsv`，代價按次數打折。回放：帶資料 詞 91.2% → 91.5%、整句持平，
  冷引擎整句 89.3% → 87.5%（兩條歧義句改讀：fayibanben 法語版本、shishao 是啥），每鍵最慢 3.5 ms → 4.7 ms
- [x] README 安裝 / 隱私 / 資料許可（2026-09-06）：pkg 安裝與「仍要開啟」、解除安裝；不上傳資料、雲聯想發什麼發到哪怎麼關、
  輸入日誌預設開只寫本機怎麼關怎麼清；測試版許可與隨包資料署名錶（與「關於」頁一致）
- [x] 領域詞拆成隨包的 `.qj`（2026-09-06）：`dict-convert lexicon` 把 THUOCL 領域詞按來原始檔拆成 11 本（語料 ≥ 50 次的留在基礎詞庫），
  基礎詞庫 22 萬 → 8.7 萬條、`dict.qj` 10 MB → 3 MB、載入 79 → 25 ms；領域詞庫合計 13 萬條 7 MB 放 `Resources/dicts/`，
  `[dictionaries] domains` 預設只開成語，偏好設定「詞庫」頁可勾選。回放（614 條冷引擎）：詞 前五 96.5% → 96.1%（少一個詞），整句持平 87.5%
- [x] pkg 安裝器（2026-09-06）：`bundle.sh --pkg` 出 `Qingjian-<版本>.pkg`（裝 `/Library/Input Methods/`，關重定位，postinstall 以登入使用者身份
  `qingjian-macos --register` 註冊並啟用輸入源，歡迎頁 / 許可證 / 結束頁），版本號與構建號打包時注入，`uninstall.sh` 隨包；
  簽名 / 公證做成環境變數開關，等 Developer ID 證書到位就能籤。未簽名的 pkg 測試者要在「隱私與安全性」裡點「仍要開啟」
- [x] 崩潰不丟（2026-09-06）：學習資料六張表、`config.toml`、`.env` 原子寫（臨時檔案 + fsync + 改名）；學習資料壞行跳過、
  編碼壞了按替換字元讀、真讀不了退回記憶體學習不覆蓋檔案，輸入法照常啟動；IMK 回撥與定時器邊界 `catch_unwind`，panic 後緩衝區字母原樣上屏；
  啟用期間每 60 秒落一次盤。設計見 `architecture.md`「崩潰不丟」
- [x] 候選視窗收尾：豎排 / 橫排（`[general] layout`）、拼音行分段樣式（Core `MarkedSegment`，糾錯刪除線的位置已留好）、
  拼音顯示位置 行內 / 視窗 / 兩處（`[general] preedit`）、按游標所在螢幕定位並處理螢幕邊緣
- [x] 日誌按天分檔案、只留 7 天；檔案被刪後下一條日誌重建（`app/logging/`，自己的 `LogFile` 寫入端按 nlink 判斷）
- [x] 偏好設定視窗（原生 AppKit，改完即寫回 `config.toml`；金鑰寫 `.env`）
- [x] 偏好設定重做（2026-09-05）：六頁標籤檢視（通用 / 候選視窗 / 快捷鍵 / 模糊音 / 詞庫 / 雲服務 / 進階），使用者視角措辭，
  快捷鍵錄製按鈕 + 「恢復預設快捷鍵」，「詞庫」頁匯入 / 開關 / 移除，編輯選單讓 ⌘V 能貼上
- [x] 產品詞庫（2026-09-05）：`assets/lexicon/` 自建源（規範字 8105 + 常用詞 5.6 萬 + THUOCL 領域詞 15.7 萬），`dict-convert lexicon` 建 `dict.tsv`，
  讀音 Unihan + LLM 多音字標註（`gloss-gen pinyin`，6.8 萬詞，含常用詞表自帶拼音的 288 處糾錯），詞頻用自己的語料統計；成品 20.5 萬條，
  `dict.qj` 10 MB、`lm.qj` 27 MB，啟動 70 ms；英文詞表換成 ESDB / CSpell 9.5 萬詞。霧凇拼音與其英文詞表已全部移除
- [x] 短語層（2026-09-12）：常用詞表是詞典詞頭，不收 我的 / 好的 / 不知道 / 有沒有 這類人整塊打的組合；`dict-convert phrases` 從語料相鄰兩三詞裡挖
  （對話語料 ≥ 2000 次 + 邊界規則，讀音由成分詞拼出）5400 條進基礎詞庫，品牌詞 青簡 也進（`brand.tsv`）；起因與驗收見 `docs/notes/phrase-layer.md`
- [x] 領域詞（2026-09-12）：輸入日誌裡選過、公開語料裡出現過、詞庫與短語層都沒有的詞人工挑 48 條進基礎詞庫（`assets/lexicon/domain_words.tsv`：對齊 / 後端 / 詞庫 / 候選框 / 看看 / 再也），
  語言模型裡與短語一樣走合成計數（低頻詞當 token 統計會吸走成分詞的二元證據）；回放詞級 +34、整句 +6、整句評測集持平，見 `docs/notes/domain-words.md`

2026-09-06 狀態：自用日常在用；`bundle.sh` 從 `assets/`（詞庫源、釋義表）與 `data/generated/`（生成物、`.qj`、領域詞庫）打產品資料，沒有生成物時打樣例；
`--pkg` 出分發包。給測試者發第一版前只剩特殊應用驗證；簽名等 Developer ID 證書。

## Phase 3 — Translation

- [x] Candidate Translation API（`Translator` trait，`Engine::annotate` 候選生成後非同步補譯文；`Sense` 詞性 + 譯文 + 讀音）
- [x] 中文 → English、中文 → 日本語：`tools/gloss-gen` LLM 批次生成 6.4 萬常用詞（詞性、譯詞、日文假名），隨包表；日文按漢欄位注平假名
- [x] 本地詞典：`Glossary` 記憶體查表，格式 `詞\t詞性. 譯詞[|假名]`；CC-CEDICT 表保留為備用來源
- [x] 翻譯語言設定：`[general] learning_language`，選單與偏好設定只列打進包裡有釋義表的語言
- [x] Option+數字上屏候選的譯文
- [x] 釋義表隨倉庫釋出（`assets/glossary/`，2026-09-05）：中→英 / 日覆蓋整個自建詞庫 24.9 萬詞，多字詞 91.6% 有英文釋義、98% 日文；
  英→中 4.5 萬詞（wordfreq ≥ 2500 + 技術詞），英文候選右側顯示中文（Engine 按候選種類查表）；三張表都進 `.qj`
- [x] 上屏譯詞：⌥ + 數字第一條、⇧⌥ + 數字第二條，修飾鍵可錄製；⌃⌥T 翻譯應用裡選中的文字（雲服務）
- [x] 釋義兜底（2026-09-06）：Core `GlossFiller` trait（與 `Predictor` 分開：獨立執行緒、攢批、不丟請求），隨包表查不到的詞庫詞 / 雲端詞**上屏後**入隊，
  `qingjian-predict::CloudGlossFiller` 攢 1.5 秒或 8 個詞發一次（提示詞與 gloss-gen 同源，只要當前學習語言），結果經 `Translator::learn` 進
  `qingjian-translate::PersonalGlossary`（`user-glossary-<語言>.tsv`，格式同隨包表，可手改，`LayeredTranslator` 個人表優先），隨 flush 落盤；
  殼每秒 `Engine::poll_glosses`；隨雲聯想開關一起開，發出去的只有那個詞
- [ ] JMdict 校對假名與日文詞性（等級標籤已做，見 Phase 4）

## Phase 4 — Learning

- [x] 使用者詞頻（`user.tsv`）、使用者詞（`user-words.tsv`）
- [x] 詞級排序接語言模型：上一個上屏的詞做上下文，同輸入串下選過的詞優先（`user-choices.tsv`），非末尾簡拼少的切分優先
- [x] 輸入習慣學習：個人 n-gram + 自動造詞；使用者點選的轉移記雙份、整句路徑順帶的記一份，選擇次數加分取對數並封頂（否則 的 的 173 次會把 號的 抬過 好的）
- [x] 個人敲錯表（2026-09-06，`user-typos.tsv`）：接受過的 (敲的, 要的) 音節對，詞圖敲錯邊與整段糾錯按次數打折；退格撤銷一併撤銷
- [x] 學習資料落盤可靠（2026-09-06）：六張表原子寫、壞行跳過、讀不了退回記憶體學習、啟用期間每 60 秒落一次盤（Phase 2「崩潰不丟」）
- [x] 輸入統計（2026-09-06）：Core `UsageMeter` trait，每次上屏折成漢字 / 中文詞（整句按切詞數）/ 英文詞 / 次數；
  `qingjian-learning::UsageStats` 按天記 `usage.tsv`，與輸入日誌無關（日誌關掉或清空統計還在），隨學習資料每 60 秒落盤；
  偏好設定「統計」頁：今天 / 最近 7 天 / 累計 三行四列 + 「累計 xx 字，約等於 n 本《某書》」（`book_scale`，書目從《道德經》到《平凡的世界》）
- [x] 生詞識別（2026-09-06）：Core `VocabularyTracker` trait + `Sense::fresh`；一條譯詞在使用者上屏時出現在候選窗口裡的輪次不到 3（`FRESH_UNTIL`）算生詞，
  候選裡橙色畫譯詞，看熟了變回灰。「看到」按上屏那一刻螢幕上那一頁算（殼 `Engine::note_displayed`），上屏帶譯詞的候選記「上屏過」，⌥+數字 打出譯詞記「用過」；
  `qingjian-learning::VocabularyBook` 一個譯詞一行 `user-vocab.tsv`（語言 / 譯詞 / 看到輪次 / 上屏 / 用過 / 首見 / 末見），只記譯詞與次數
- [x] 學習詞彙統計（2026-09-06）：偏好設定「統計」頁詞彙塊：見過 / 看熟 / 上屏過 / 打出過 / 本週新見的譯詞數（`VocabularySummary`），
  再按等級分行（詞表總數 / 見過 / 看熟 / 上屏過）：英文 CEFR A1–C2（CEFR-J 1.5 + Octanove，8,845 詞）、日文 JLPT N5–N1（Tanos 經 elzup，7,757 詞），
  `qingjian-translate::LevelTable` 讀 `assets/levels/levels-{en,ja}.tsv`（`tools/corpus/levels.py` 生成），`VocabularyBook::with_levels`；等級不進候選視窗
- [ ] 可選複習功能

## Phase 5 — Cross-platform

- [~] Windows TSF（`apps/windows/{server,tsf}`，真機自用中；細節見 `docs/design/architecture.md`「Windows：TSF」）
  - [x] Server 程序 + TSF DLL 骨架、命名管道 IPC、多會話分派、端到端上屏
  - [x] preedit 內聯下劃線、候選窗（詞性 + 譯文 + 分頁 + 陰影）、雲聯想、失焦上屏、中英切換、設定介面、Inno 安裝器
  - [x] 候選窗渲染搬進 Server 程序 + `uiAccess` + 自籤，覆蓋微軟商店 / 工作列搜尋等高 z-band 宿主
  - [x] 中英切換可配置（2026-09-15，issue #81）：`[shortcut] switch_mode` 選單擊 Shift / 單擊 Ctrl / 不切換，
    `[general] english_mode` 關掉則內建英文模式整體停用（固定中文、不再登記語言欄按鈕、狀態條也不切）；
    四項都在「設定 → 通用」，DLL 啟用時讀一次配置
  - [ ] 發版：Certum 開原始碼簽名證書、`windows-v<版本>` 標籤與 CI
- [x] Linux Fcitx5 第一階段：Rust Server、預設候選面板、使用者目錄安裝與手動啟動（2026-09-18）；未改系統 Fcitx5 的 GTK4 / Qt6 X11 輸入已驗證
- [ ] Linux 後續：Server 自繪 X11 視窗、Server 向 GNOME 擴充套件傳送點陣圖、神經重排、自動啟動與 Debian 包；各自獨立 PR，IBus 暫緩
- [ ] 配置同步
- [ ] 跨平臺詞庫

## Phase 6 — 雲聯想

Core 定義 `Predictor` trait（與 `Translator` / `Learner` 同一模式），網路實現放獨立 crate `qingjian-predict`，
Core 永遠不聯網。第一個實現接 DeepSeek（OpenAI 相容介面），介面做成 provider 無關。

- [x] 本地輸入歷史：`InputHistory`（記憶體環形 4096 字元，可清除；落盤與檢視介面待做）
- [x] `Predictor` trait：`submit` / `poll` 非阻塞，請求帶序號，Engine 只認最新序號；`NoPredictor` 預設
- [x] `qingjian-predict`：`CloudPredictor`，async-openai 走 OpenAI 相容介面，後臺執行緒防抖 300 ms、快取 64 條、超時 5 s
- [x] 雲端詞：模型給「拼音對應的詞 + 全拼」，Core 逐音節校驗；詞庫沒有的選中後記成使用者詞（`user-words.tsv`）
- [x] 雲端詞位置（2026-09-05）：到了只補進第一頁末尾 `[predict] slots` 格（預設 2），前面的本地候選一格不挪，不預留不畫佔位；
  Core `CandidateLayout` 出排布，殼 `Session` 只管高亮與頁碼；偏好設定「雲服務」頁可選 0–4 格
- [x] 觸發：只在組句中，停鍵後同一次請求要雲端詞和整句補全（preedit 右側，Tab 接受）；上屏後不聯想
- [x] 整句補全進學習（2026-09-05）：Tab 接受的句子按語言模型切詞（`sentence::segment_text`）記進個人 n-gram，可隨退格退回
- [x] 糾錯：雲端詞按容錯編輯距離校驗（簡拼不算錯），模型可糾正錯字 / 漏字 / 多字；本地歷史不進請求
- [x] 提示策略（2026-09-05）：本地候選與本地整句在提示詞裡標成「可能全錯的本地猜測」，模型獨立判斷、只給本地給不出的詞，沒有就空；不再照抄本地候選
- [x] 上下文來源：IMK `attributedSubstringFromRange:` 讀 marked text 前後，應用不支援時退回本地輸入歷史
- [x] 展示：雲朵圖示（SF Symbols `cloud`）區分，**不重排已有候選、不阻塞輸入**
- [x] 隱私：預設關閉；開啟時選單欄「中 / 英」帶 ☁︎；Secure Input 絕不傳送；觀察長度在 Core 裁剪
- [x] 配置：`~/Library/Application Support/Qingjian/config.toml` 的 `[predict]`，首次執行寫模板；金鑰可填配置或環境變數
- [x] 問字模式：`?` + 問題拼音，`PredictionKind::Question` 走同一條 Predictor 通道、單獨的提示詞，答案帶聲調讀音、不校驗拼音（見 candidate-ui.md）
- [x] 問字只答字不復述（2026-09-05）：請求帶本地整句轉換出的問題漢字（`guess`），提示詞只答被問的字，`restates_question` 剔掉把問題寫回來的「答案」
- [x] 選單開關、偏好設定窗口裡的開關 / 介面 / 模型 / 金鑰
- [x] 釋義表進 `.qj`（2026-09-05）：`Glossary` 雙存法（TSV 雜湊表 / 對映的 arena + 詞條表 + 釋義表 + 雜湊索引），`pack glossary --language`，啟動回到 50 ms
- [x] 翻譯選中文字讀不到選區時候選視窗提示 2.5 秒（`host/presenting/notice.rs`）
- [x] 語料挖新詞（`dict-convert mine`）：分詞落成連續單字的段按子串計數，虛詞規則 + 相鄰字對 PMI≥3 過濾（2026-09-07 從會話指令碼進工具，`oov_filter.rs`），`lexicon --extra-words` 併入詞庫
- [x] 多詞庫與詞庫管理（2026-09-05）：Engine 附加詞庫列表；使用者目錄 `dicts/` + `[dictionaries] disabled`；偏好設定「詞庫」頁匯入（TSV / Rime yaml / .qj → .qj）、開關、移除
- [x] 翻譯選中的文字（2026-09-05）：⌃⌥T 把應用裡的選區交給雲端翻譯，譯文在候選窗口裡回車替換、Esc 保留；快捷鍵可改；
  雙向：中文選區譯成學習語言，外文（拉丁字母 / 假名）選區譯回中文，方向按字元統計定（`translation_target`）
- [x] 輸入歷史落盤與清除（2026-09-05）：`input-log.jsonl` + 偏好設定「進階」頁開關 / 清空（檢視介面沒做，檔案是 jsonl 直接看）
- [ ] 金鑰進鑰匙串（等簽名定了再做）；雲聯想按應用停用
- [ ] 請求量控制：按分鐘限流、統計本月請求數並顯示

## Phase 7 — 個人模型

目標是「輸入法越用越像自己」。分兩步，第二步有評測門檻。

- [x] 個人 n-gram：bigram 計數 + 與靜態模型按證據量插值，隨上屏線上更新，直接進整句轉換的 Viterbi 打分（見 Phase 2）。
  個人資料量級（一年幾百 KB 到幾 MB）上，這一步拿到絕大部分個性化收益，成本接近零
- [x] 個人 trigram（2026-09-06）：與二元一起線上計數，絕對摺扣（D = 0.75）回退到二元，不訓練；`--replay` 在 423 條日誌上與二元持平
  （冷 詞 77.7% / 整句 87.0%，帶資料 90.2% / 97.1%），三元要資料攢起來才見效，`--typing` 每鍵最慢 2.7 ms 不變
- [x] 小 Transformer 實驗分支（2026-09-08）：`qingjian-neural`（candle，Metal / Accelerate / CPU），用途「重排整句轉換的前 6 條路徑」；
  CLI `--neural` 接，殼未接。回放上詞 +0.5、整句 −1.1（靜態替換 λ 0.25、前文 64），沒過門檻，且尺子有偏，見 `docs/notes/neural-rescoring.md`
- [x] 整句評測集上過門檻（2026-09-08 晚）：凍結集 8322 句，基線整句首選 37.6%，small λ 0.75 前文 64 到 41.9%（+4.3），
  不給前文也有 +2.2；翻好 382 句、翻壞 54 句，見 `docs/notes/neural-rescoring.md`
- [x] 進殼（2026-09-08 晚）：前文 KV 快取（64 字前文 × 8 條 133 → 28 ms）、Core `engine/rescoring` 非同步重排（後臺執行緒 + 文本分數快取，殼停鍵 80 ms 請求）、
  應用游標前文當前文、λ 預設 0.5、`[model] enabled` 開關與「雲服務」頁勾選、知微模型現隨包放 `Resources/models/hanzhang-zhiwei/`；TextEdit 端到端 候選聲稱 → 候選生成
- [x] 模型單檔案 `.qjm`（2026-09-12）：`.qj` 容器新種類 `Kind::Model`，三節原樣裝三件套；`find_model` 先 `.qjm` 再目錄；`pack model` / `tools/release/pack-model.sh`；
  data Release 上傳 `model.qjm`，mac `Resources/models/hanzhang-zhiwei/` 與 Windows `data\models\hanzhang-zhiwei` 只帶這一個檔案
- [ ] 本地模型後續：個人微調（閒時訓練）；權重許可已定與程式碼一致 GPL-3.0-or-later（2026-09-12，寫在 `pack-model.sh`）
- [ ] 閒時訓練：門禁包括接電源、溫度、空閒時長；訓練資料來自本地輸入歷史；模型與資料都可一鍵清除
- [ ] 評測門檻：留出使用者文本上比 n-gram 的困惑度與 top-1 命中率，贏了才預設啟用；每次按鍵推理延遲有上限
- [ ] 模型檔案的版本與遷移

## 規模參考

| 專案 | 量級 |
|---|---|
| librime（C++） | 約 5 萬行 |
| Phase 1 + 2 到 macOS 自用級別 | 全職約 2 到 3 個月 |
