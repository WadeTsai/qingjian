# 架構

## 總體結構

```text
                    Qingjian Core
                         │
        ┌────────────────┼────────────────┐
        ▼                ▼                ▼
   macOS Adapter    Windows Adapter    Linux Adapter
       IMK               TSF          IBus / Fcitx
        │                │                │
        ▼                ▼                ▼
  Candidate UI      Candidate UI      Candidate UI
```

詞庫、拼音解析、候選生成、排序、使用者詞頻學習和翻譯能力全部屬於 Core。
平臺層只做兩件事：把系統輸入事件翻譯成 Core 的輸入，把 Core 返回的候選畫到候選視窗。

判斷標準：把 IMK 換成 TSF，不應該需要改 Core 的任何一行。

## 架構約束

這些是核心設計決定，不要違反。

1. **Core 平臺無關。** `qingjian-core` 及其兄弟 crate 不允許依賴任何平臺 API。
   平臺層裡不允許出現排序邏輯、詞庫訪問或翻譯呼叫。
2. **一個候選詞只顯示一種輔助語言。** 使用者配置 Primary Language + 單個 Learning Language。
   不要設計成 `translations: Vec<Translation>` 或 `HashMap<Lang, String>` 這類多語言並列的資料結構，
   那會在 API 層面把「一次只學一種語言」這條產品原則給破壞掉。
   翻譯是候選詞的 annotation（可選、單條），不是並列的第二套候選系統。
3. **輸入優先於學習。** 任何為學習功能增加的延遲、彈窗、UI 干擾都是設計錯誤。
   翻譯查詢不能阻塞候選生成，Core 必須能在翻譯尚未就緒時先返回候選。

## Workspace 結構

```text
qingjian/
├── crates/
│   ├── qingjian-core/          # composition / parser / correction / candidate / ranking / sentence / engine …（下面單列）
│   ├── qingjian-dictionary/    # 詞庫載入與查詢
│   ├── qingjian-translate/     # 候選翻譯 annotation
│   ├── qingjian-learning/      # 使用者詞頻、使用者詞、個人英文詞、個人 n-gram、個人敲錯表（user.tsv / user-words.tsv / user-english.tsv / user-ngram.tsv / user-typos.tsv）、輸入日誌（input-log.jsonl）、輸入統計（usage.tsv）、詞彙記錄（user-vocab.tsv）
│   ├── qingjian-predict/       # 雲聯想：Predictor 的網路實現（OpenAI 相容介面）
│   ├── qingjian-lm/            # 整句轉換的 bigram 語言模型：LanguageModel 的實現
│   ├── qingjian-neural/        # 字級 Transformer 的本地推理（candle）：SentenceScorer 的實現，給整句前幾條路徑重打分
│   ├── qingjian-format/        # .qj 資料容器：mmap 開啟、零複製檢視、寫入器、可落盤的雜湊索引（dictionary / lm 依賴它）
│   ├── qingjian-render/        # 自繪渲染器：候選窗一幀 + 主題 → 點陣圖，各平臺貼圖
│   ├── qingjian-update/        # 檢查更新：讀 releases.json、驗 ed25519 簽名、按平臺與渠道挑新版本
│   └── qingjian-platform/      # 平臺層共用的部分：配置檔案、協議型別
│
├── apps/
│   ├── cli/                    # 測試工具：查詢、逐鍵計時、輸入日誌回放評測、整句評測
│   ├── macos/                  # IMK 輸入法殼（app / host / imk / candidates / menubar / preferences）
│   ├── windows/                # 同一個產品的四個 package：server（IPC 分派 + Engine + 命名管道 + 自繪候選窗）、
│   │                           #   tsf（TSF 文本服務 DLL：COM 鏈路 + 連 Server 的管道客戶端）、settings（WinUI 3）、installer
│   └── linux/                  # server（Engine + Unix socket）+ fcitx5（C++ 外掛）+ scripts
│
├── tools/
│   ├── dict-convert/           # 產品資料生成：lexicon / bigram / mine / english / emoji / pack
│   ├── gloss-gen/              # LLM 批次生成釋義表與多音字標註
│   └── corpus/                 # 語料預處理指令碼（uv）
│
├── assets/                     # 隨倉庫的產品資料來源：詞庫源、釋義表、emoji 表、詞彙等級表（levels/，CEFR-J / Octanove / JLPT）、圖示、樣例
├── data/                       # gitignore：語料、Unihan、生成物 data/generated/
├── docs/
└── README.md
```

`qingjian-core` 內部模組：

```text
qingjian-core
├── composition     # 輸入狀態機：拼音緩衝、游標、上屏
├── parser          # 拼音切分（全拼 / 簡拼 / 雙拼 / 模糊音）
├── candidate       # 候選資料模型（Candidate / Translation / Sense / PartOfSpeech / Language）；layout 是分頁排布（本地候選 + 雲端固定槽位），各平臺殼共用
├── ranking         # 候選排序
├── shortcut        # 快捷候選：日期 / 時間 / 星期、v 表示式模式（四則運算、中文數字），不查詞庫
├── english         # 英文模式候選：詞表精確詞 / 字首補全 / 一處編輯糾正（edit.rs），大小寫跟著敲的走
├── sentence        # 離線整句轉換：詞圖 + bigram Viterbi + 束搜尋，LanguageModel trait（qingjian-lm 實現，預設退化為一元），UserNgram 個人 n-gram（二元 + 三元），Context 上文（前兩個詞），
│               #   SentenceScorer trait（qingjian-neural 實現）：convert_paths 出前 K 條路徑，Engine（engine/rescoring）按 路徑分 + λ·(神經分 − 靜態分) 重排，非同步時後臺執行緒打分、殼停頓後取
├── emoji           # emoji 候選：EmojiTable（詞 → emoji，Unicode CLDR 中文 annotations）
├── fuzzy           # 模糊音：FuzzyRules（配置 [fuzzy]）把每個音節擴充套件成多種寫法，Expanded 借出給詞庫多寫法查詢
├── shuangpin       # 雙拼：Scheme 七套方案的鍵位表，decode 把敲的鍵解成全拼（音節間帶 '），Decoded 把上屏消耗換算回鍵數；切分之後全部複用全拼
│                   # engine::ModeKeys（配置 [shortcut]）：表示式 / 問字字首鍵，只能是 v / u / i；問字鍵後跟十六進位制出碼點字元
├── engine          # 對外門面：Engine，以及 Translator / Learner trait 與空實現
└── storage         # 小檔案落盤原語：write_atomic（臨時檔案 + fsync + 改名）、read_text_lossy；學習 crate 與配置都用它
```

詞庫記憶體佈局（`qingjian-dictionary`）：詞文本與拼音鍵各放一個連續 arena，詞目只存 `u32` 偏移 + 詞頻，
鍵按位元組序排好（當年 89 萬條的測試詞庫約 150 MB RSS，比 `String` + `Vec<String>` 的樸素佈局省三分之二；現在產品詞庫 8.7 萬條，
且 `.qj` 是 mmap 直接對映，見「資料檔案」）。
查詢介面是 `lookup_pattern(&[SyllablePattern])`（命中音節數 ≥ 模式長度）與 `lookup_exact`（正好等長），每個位置可以是
完整音節或字首 / 聲母。實現是逐級字首收窄：「以某段字首開頭的鍵」在排好序的索引裡總是連續區間，完整音節直接二分到
`字首 + 音節 + 空格`，簡拼位置按區間裡實際出現的音節跳塊（每塊看第一條鍵就能二分出塊尾），區間小於 48 條就改線性比對；
代價與匹配到的音節組合數成正比，與首音節下有多少鍵無關。每個位置可以給多種寫法（`lookup_pattern_alt` / `lookup_exact_alt`，
模糊音用），同一位置的寫法在每級逐個走、代價相加不相乘；呼叫方保證同一位置的寫法互不覆蓋。Engine 側同一次查詢裡相同的字首模式只查一遍，
排序鍵預計算、遠超 500 條時先 `select_nth` 再排，候選最多給殼 500 條。

拼音切分（`parser`）：按位置做動態規劃，每個位置只保留最優 8 種字首切分，token 可以是完整音節、
聲母（簡拼）或末尾未打完的字首。排序鍵：音節少 > 不完整音節少 > 前面的音節長。

中英混輸（`Engine::insert_english`）：整段輸入（不含 `'`）在英文詞表裡就加一個 `CandidateKind::English` 候選，
上屏吃掉整段輸入。排第一的條件：切不動（有未切分尾部），或最優切分除末尾外還有不完整音節
（`hello` → `he l l o`）；否則排第二（`china` 是乾淨的 `chi na`）。詞表 `WordList` 在 dictionary crate。

程式碼組織約定（2026-09-03 起）：一個 struct / enum / trait 及其 impl 單獨一個檔案，
模組檔案只做 `mod` 宣告、re-export 和自由函式；結構體欄位逐條 `///` 註釋並用空行分隔；
`thiserror` 的 `#[error]` 文案用英文，日誌與 UI 文案用中文。

`Engine` 的會話 API：`set_input / push / backspace` 喂拼音，`query()` 返回不帶譯文的
`Query { segmentations, candidates, timings }`，`annotate(&mut CandidateList)` 補譯文，
`commit(&Candidate)` 上屏並餵給 Learner（詞頻、詞轉移、自動造詞）。`Learner::flush()` 由殼在退出 / 停用時呼叫，
失敗只記日誌不返回錯誤；殼停用時還調 `break_chain()`，之後上屏的詞按句首記；
`Engine::flush_learning()` 把學習資料與輸入日誌一起落盤且不作廢格子快取，殼啟用期間也定時調它。

### 崩潰不丟：原子寫、損壞容忍、panic 隔離

輸入法程序隨時會被 launchd 殺掉或自己崩掉，使用者攢的學習資料和正在打的字都不能因此沒了：

- **原子寫**（`qingjian_core::storage::write_atomic`）：學習 crate 的六張 TSV、輸入統計 `usage.tsv`、詞彙記錄 `user-vocab.tsv`、`config.toml`、`.env` 都先寫同目錄的臨時檔案，
  flush + fsync 後改名覆蓋；任何時刻磁碟上要麼是舊檔案要麼是新檔案。輸入日誌 `input-log.jsonl` 是追加寫不走這條路，
  崩潰最多留半行，回放工具按行跳過壞行並計數。
- **損壞容忍**：學習資料各檔案按行解析，格式不對的行記一條警告跳過（下次落盤就清掉了），編碼壞掉的位元組按替換字元讀進來；
  只有許可權、壞盤這類真正的 io 錯誤才算讀失敗，這時殼退回只在記憶體裡學習（不帶路徑，不會拿空表覆蓋使用者的檔案），輸入法照常啟動。
- **panic 隔離**：`define_class!` 生成的 IMK 回撥是 ObjC 執行時直接調的，panic 穿出去程序就沒了。殼在按鍵處理、
  `commitComposition`、`activateServer` / `deactivateServer`、定時器這些邊界都用 `catch_unwind` 攔住（`imk::catch_panic`），
  攔下後把緩衝區裡的字母原樣交給應用、清引擎狀態、收視窗（`imk::recover_from_panic`），按鍵交還給應用；
  `main.rs` 裝的 panic hook 只記位置與 backtrace 進日誌。
- **回撥重入**：`Host` 是主執行緒 `RefCell` 單例（`host::with`）。閉包裡凡是碰應用那邊的東西（`surrounding_text` 讀上下文、
  `caret_rect` 取游標、`insert_text`）都要等應用回話，IMK 在等的時候會跑一輪 run loop，`deactivateServer:` 之類的回撥就可能在
  借用期間進來（2026-09-07 真發生過：`refresh → request_prediction → surrounding_text` 期間收到 deactivate，`borrow_mut` panic，
  善後再 panic，程序退出重啟）。規矩是 IPC 一律放在 `with` 之外、分兩次借；`with` 本身用 `try_borrow_mut`，借不到記 warn 返回 `None`，
  再有漏網的重入也只是跳過一次呼叫，程序不死。
- **有界丟失**：學習資料除了停用時儲存，啟用期間借每秒看配置檔案的定時器每 60 秒 flush 一次（沒有新資料時是空操作），
  被殺最多丟一分鐘的學習。

實際結構會隨開發調整，調整後同步更新這裡。

## crate 依賴方向

```text
qingjian-dictionary        （純資料載入與查詢，依賴 qingjian-format 的 .qj 容器）
        ▲
qingjian-core              （定義 Translator / Learner / Predictor trait，依賴 dictionary）
        ▲           ▲            ▲
qingjian-translate  qingjian-learning  qingjian-predict  qingjian-lm  qingjian-neural   （實現 core 的 trait，依賴 core；learning 另依賴 translate 的 LevelTable 做詞彙按級彙總）
        ▲           ▲            ▲
qingjian-platform          （配置檔案 Config：general / shortcut / fuzzy / predict 分節，toml_edit 原地改鍵保留註釋；協議型別，可序列化；依賴 core、predict）
        ▲
apps/*                     （組裝：Engine::new(dict).with_translator(..).with_learner(..).with_predictor(..)）
```

`apps/cli` 是 Phase 1 的測試殼：`cargo run -p qingjian-cli -- kaifa` 直接查詢，
不帶引數進入互動模式（拼音查詢、序號上屏、`:q` 退出），`--user-dict` 指定使用者詞頻檔案，
`--language en|ja|es|vi` 或環境變數 `QINGJIAN_LEARNING_LANGUAGE` 選學習語言。
`--typing` 是效能測試模式：把輸入當一鍵一鍵敲進去，每個字首查一次並標註譯文，一行一鍵列印各階段耗時
（這是輸入法每鍵的真實工作量，聯想在後臺執行緒不算），啟動日誌裡帶各資料檔案的載入耗時。
效能改動要用 release 構建跑它看數字，目標每鍵 10 ms 以內。

- Core 只依賴 dictionary，不依賴 translate 和 learning。翻譯與學習通過 trait 注入（`Translator` / `Learner` / `InputLogger` / `UsageMeter` / `VocabularyTracker`，
  預設實現都是空操作），這樣 Core 的單元測試和 CLI 工具不需要真實詞典也能跑。
- `qingjian-platform` 裡的型別必須可序列化（serde）：macOS 上 Core 與殼同進程，
  Windows 和 Linux 上 Core 在獨立 Server 程序，同一套協議型別兩邊都用。
- `storage` 只放 Core 自己的持久化原語，使用者詞頻的資料模型歸 `qingjian-learning`。

## 翻譯的非同步模型

Core 的會話 API 分兩步返回：`update(input) -> CandidateList` 立即返回不帶譯文的候選；
譯文由 translate 在後臺查表，通過 `poll_annotations()` 或回撥補上。
本地查表通常在一次事件迴圈內就緒，但介面上必須允許「候選先到、譯文後到」，
平臺層收到 annotation 更新後只重繪對應行。

## Core 的關鍵技術決定

### 多詞庫

Engine 查詞的詞庫是一個列表：主詞庫（隨包 `dict.qj`）、附加詞庫（`Engine::set_extra_dictionaries`）、使用者詞（Learner 持有）。
三者一起進詞級查詢和整句詞圖；附加詞庫不帶語言模型，它的詞在路徑上按詞頻兜底打分（`sentence::fallback_log_prob`），
所以領域詞庫、匯入的第三方詞庫隻影響「有沒有這個詞」和它的詞頻，不改變語言模型的尺度。
**領域詞拆開**（2026-09-06）：`dict-convert lexicon` 把 THUOCL 領域詞按來原始檔拆成 11 本 `dicts/<領域>.qj`（法律 / 醫學 / 地名 / 成語 /
詩詞名句 / IT / 財經 / 飲食 / 動物 / 汽車 / 歷史人物，各帶 META），只有語料裡出現 ≥ 50 次的領域詞（`--domain-keep-min`）留在基礎詞庫
（它們其實是通用詞：醫療器械、侵權行為）；基礎詞庫從 22 萬條降到 8.7 萬條、`dict.qj` 10 MB → 3 MB，領域詞庫合計 13 萬條 7 MB。
分詞統計語料時仍把 `dicts/*.tsv` 一起當詞表，詞表與拆分前一致，語言模型不用重跑。
殼負責裝配，附加詞庫有兩處：隨包的領域詞庫在 `.app` 的 `Resources/dicts/`，預設關閉，配置 `[dictionaries] domains` 列出開啟的
（預設只有 `idioms`，偏好設定「詞庫」頁可勾選、不能移除）；使用者自己匯入的放使用者目錄 `dicts/`（macOS 是 `~/Library/Application Support/Qingjian/dicts/`），
目錄裡的 `.qj` / TSV 檔案全部載入，配置 `[dictionaries] disabled` 列出要關掉的檔名；匯入 = `qingjian_dictionary::import`
把青簡 TSV / Rime `.dict.yaml` / `.qj` 轉成 `.qj` 放進去（Rime 的 YAML 頭只取 `name:`，權重非整數當 1），
移除 = 檔案挪到 `dicts/removed/`，開關 = 改配置，三個動作之後 `Host::reload_dictionaries` 重新裝配。這也是第三方詞庫帶著自己許可證單獨分發的落點：
`.qj` 的 `META` 裡有名稱與許可證，偏好設定裡直接顯示。

### 資料檔案：`.qj` 容器

詞庫、語言模型這類常駐資料用自己的二進位制容器 `.qj`（`crates/qingjian-format`），原則是**記憶體佈局就是檔案佈局**：
從 TSV 解析出來的幾段連續陣列（詞庫的詞文本 arena、拼音鍵 arena、鍵索引、詞目；語言模型的詞 arena、詞表、雜湊索引、
CSR 偏移與後繼）原樣落盤，開啟時 mmap 整個檔案、校驗一遍頭與分節邊界，不反序列化。啟動從 0.9 s 降到 50 ms。

- 檔案 = 32 位元組頭（魔數 `QINGJIAN`、格式版本、資料種類 `Kind`、分節數）+ 分節表（4 位元組標籤 + 偏移 + 長度，正文 8 位元組對齊）
  + 各分節。第一節固定是 `META`：TOML 的 `Metadata`（名稱、許可證 SPDX、署名、來源、版本、條數、生成者），
  偏好設定裡的詞庫列表直接顯示它，第三方詞庫各帶各的許可證靠的就是這一節。
- 資料種類：詞庫、語言模型、釋義表、emoji 表、英文詞表，以及本地整句模型 `Kind::Model`——副檔名換成 `.qjm`，
  三節 `CONF` / `VOCB` / `SAFT` 原樣裝匯出的 `config.json` / `vocab.json` / `model.safetensors`（safetensors 是不透明載荷，
  mmap 後切片給 candle，張量搬上裝置後容器即丟；`META.entries` 記引數量）。`qingjian-neural::find_model(dir)` 先找 `.qjm`、沒有再認三件套目錄，
  所以開發直接載入訓練直出的目錄，隨包與使用者目錄只有一個檔案；`dict-convert pack model`（`tools/release/pack-model.sh` 帶後設資料調它）打包。
- 數值小端、原生對齊，crate 在大端機器上拒絕編譯。字串分節開啟時校驗一次 UTF-8，之後 `Text::deref` 走 unchecked
  （曾經每次 deref 都重新校驗 30 MB，CLI 直接卡死）。定長結構體用 `zerocopy` 派生，`#[repr(C)]` 且手工排欄位消滅填充
  （`KeyIndex` 16 位元組、`Slot` 12 位元組、`WordEntry` 12 位元組、`Successor` 8 位元組）。
- 兩種檢視：`Table<T>`（`Owned(Vec<T>)` / `Mapped`，`Deref<Target = [T]>`）與 `Text`（`Owned(String)` / `Mapped`，
  `Deref<Target = str>`）。解析路徑與對映路徑產出同一種結構，查詢程式碼不區分。
- 檔案裡的雜湊索引（`qingjian_format::hash`）：開放定址、槽裡放條目編號、鍵留在 arena；雜湊函式必須跨程序、跨版本穩定
  （寫檔案的程序和讀檔案的程序算出來要一樣），用 FNV-1a 64 加 fmix64 終混（FNV 低位對 UTF-8 中文這種位元組模式相近的短串分佈差，
  只用低位選槽會長鏈）。**為什麼手寫而不是用庫**：標準庫與 foldhash 的雜湊器帶隨機種子，不能用；blake3 / SHA 這類密碼學雜湊
  一次幾百納秒、且是為抗碰撞設計的，這裡每鍵要算幾千次、只要分佈均勻；xxh3 / wyhash 這類非密碼學庫能用，但任何依賴升級
  悄悄改了演算法（或換了預設種子）就會讓使用者機器上所有 `.qj` 失效，而這個函式總共 12 行、有測試釘死輸出值，
  自己寫風險最小。若以後碰撞或分佈出問題，換成 xxh3 並把 `FORMAT_VERSION` 加一。
- 寫檔案先寫同目錄 `.qj.tmp` 再改名；資料檔案只整體替換，從不就地修改（mmap 的安全前提）。
- 生成：`cargo run --release -p qingjian-dict-convert -- pack dict --name … --license … --source …` → `dict.qj`，
  `pack lm --name … --license …` → `lm.qj`（`bundle.sh` 在 TSV 比 `.qj` 新時自動重打）。`Dictionary::from_path` 按魔數自動選
  `.qj` / TSV 路徑，`BigramModel::from_path`（`.qj`）與 `from_paths`（TSV）分開；輸入法與 CLI 有 `.qj` 就用它。
  釋義表、emoji 表、英文詞表還是 TSV（載入各 20 ms 以內，等有需要再進容器）。
- 格式版本不相容時 `FORMAT_VERSION` 加一，讀舊版的程式碼按需保留；`Kind` 編號只增不改。

### 候選生成需要整句轉換

詞級候選（trie 查詞庫）只能做到「能用」。日常可用的門檻是整句轉換：
bigram 語言模型 + Viterbi，加上簡拼、模糊音、雙拼。沒有整句輸入，開發者自己都不會切換過來用，
學習功能就沒有承載體。

詞庫與語言模型不自造，見 [landscape.md](landscape.md) 的資料來源一節。

### 翻譯只是 annotation

翻譯 annotation 只對詞典詞候選有意義。整句引擎產出的候選多數不是詞典詞
（一整句，或者「的」這種單字），引擎越好，能標註的候選反而越少。

待確認的策略：
- 只給詞典詞候選標註，句子候選和單字虛詞留空。
- 一詞多義（開發 → develop / development）只取最高頻義項，不展開。

### 翻譯資料本地化

翻譯走本地查表，執行時不調網路。這樣天然滿足「翻譯不阻塞候選」的約束。
隨包的釋義表由 `tools/gloss-gen` 用 LLM 離線批次生成（2026-09-04 定的路線，不用有道等網頁介面：逆向介面不穩，
攢下來的結果再分發有版權問題；LLM 輸出許可乾淨）：從我們自己統計的一元詞頻表挑常用詞（次數 ≥ 200、不超過 4 個字，約 6.4 萬詞），
每個詞一次請求同時要「詞性 + 英文譯詞 + 日文譯詞與假名讀音 + 越南文譯詞」，結果 JSONL 可續跑，`export` 轉成 `glossary-en.tsv` / `glossary-ja.tsv` / `glossary-vi.tsv`。
釋義格式 `詞\t詞性. 譯詞\t詞性. 譯詞`，日文譯詞後 `|假名`；`Sense.reading` 存整詞假名，`Sense::furigana()`（Core `candidate::furigana`）用譯詞裡的假名段當錨點把讀音對到各段漢字上，
候選視窗按 `開発(かいはつ)する` 顯示，假名淡色；純假名 / 片假名詞不注，對不上就整體注在後面。不用羅馬音。
CC-CEDICT 表（`dict-convert cedict`）保留為備用來源，覆蓋面廣但沒有詞性、釋義偏長。
執行時查不到的詞走同一條 LLM 通道補進使用者目錄的個人釋義表（**釋義兜底**）：Core `GlossFiller` trait 與 `Predictor` 分開注入——聯想是「最新請求優先」、
防抖會丟舊請求，兜底恰恰要每個詞都問到、慢點沒關係。詞庫詞 / 雲端詞**上屏後**發現隨包表沒有就入隊（只在雲聯想開著時，發出去的只有那個詞），
`qingjian-predict::CloudGlossFiller` 在獨立執行緒攢 1.5 秒或 8 個詞發一次請求（提示詞與 gloss-gen 同源，只要當前學習語言），問過的本程序內不再問；
殼每秒 `Engine::poll_glosses` 把結果經 `Translator::learn` 寫進 `PersonalGlossary`（`user-glossary-<語言>.tsv`，格式同隨包表，可手改），
`LayeredTranslator` 個人表優先疊在隨包表上，隨 `flush_learning` 原子落盤。`Translator::translate` 本身仍不聯網。

### 聯想與個人模型的邊界

- 整句轉換裡最貴的一步是詞圖格子查詞（每個簡拼位置都要在詞庫裡逐音節塊收窄）。敲鍵是增量的，第 n+1 鍵只新增以它結尾的
  最多 8 個格子，所以格子候選放在 `sentence::SpanCache`（鍵是格子模式含模糊寫法，值是排好截好的 `SpanWord`）跨按鍵複用；
  候選與詞庫、使用者詞、選擇次數、個人出現次數有關，Engine 在 commit / `learner_mut` / 換 Learner 時整個清掉。
  拼寫糾錯的上千個變體先過無分配的 `parser::is_fully_segmentable`，剩下幾個才做真正的切分。
- 整句轉換的語言模型通過 `LanguageModel` trait 注入（`sentence/language_model.rs`）：`log_prob(previous, word)`，
  模型不認識的詞返回 `None`，Core 用詞庫詞頻兜底並扣分。`qingjian-lm::BigramModel` 從 `lm.qj`（或 `lm-unigram.tsv` / `lm-bigram.tsv`）
  載入：詞表是 arena + 定長條目 + 檔案裡的開放定址雜湊索引；二元按前詞分組成 CSR（`offsets[v]..offsets[v+1]` 是 v 的後繼段，
  段內按後詞編號二分，一次查詢落在一兩個快取行裡），P(w|v) = 0.8·c(v,w)/c(v) + 0.2·c(w)/N。
  資料由 `dict-convert bigram` 統計：用青簡詞庫做一元最大機率分詞（與詞圖同一套詞表），連續漢欄位為句，`<s>` 句首標記。
  詞圖每格只留詞頻前 6 個詞（有簡拼位置的格子留 20 個：`h` 下幾十個常用字，留少了句子裡要的那個進不來），每個位置束寬 8。
  簡拼位置就是字首模式（`SyllablePattern.complete = false`），詞庫層不區分；每條路徑覆蓋的簡拼位置相同，不需要額外罰分。
- **個人 n-gram**（`sentence/user_ngram.rs` 的 `UserNgram`，由 Learner 持有、`Learner::user_ngram()` 暴露）：
  上屏的詞序列轉移計數，二元 (前詞, 後詞) 與三元 (前二詞, 前詞, 後詞) 一起記（整句按路徑上的詞逐條記，連續選詞也記；
  標點、透傳、回車上屏拼音、切應用打斷鏈，下一個詞按句首 `<s>` 記，句首詞不記三元）。上文是 `sentence::Context`（前一個詞 + 再前一個詞），
  Engine 的 `CommitChain` 記最近兩個上屏的詞；Viterbi 不擴狀態，前二詞取前驅節點的回指（它那條最優路徑上的前一個詞），是近似。
  打分時與靜態模型（只看前一個詞）插值：P = (1−μ)·P_靜態 + μ·P_個人，μ = c(v)/(c(v)+8) 封頂 0.5，前詞沒見過就不插值。
  P_個人 先算二元 P₂ = 0.8·c(v,w)/c(v) + 0.2·c(w)/N；這對上文 (u,v) 見過時再套一層絕對摺扣的三元
  P₃ = max(c(u,v,w) − D, 0)/c(u,v) + D·N₁₊(u,v,·)/c(u,v)·P₂（D = 0.75），沒見過的接續只拿回退的份額，(u,v) 沒見過就是 P₂。
  三元不訓練、不平滑引數，就是線上計數；它分辨的是二元混在一起的接續（「我想 → 去」與「不想 → 要」）。
  封頂保證沒見過的接續最多打折、不會被壓死；K = 8 讓一次誤選翻不過強 bigram，選兩次才翻。
  個人出現次數也參與詞圖每格的前 6 選擇，保證使用者常用的同音詞進得了格子。二元 + 三元超過 20 萬條時所有計數減半。
  持久化在 `qingjian-learning` 的 `user-ngram.tsv`：三列 `前詞\t後詞\t次數` 是二元，四列 `前二詞\t前詞\t後詞\t次數` 是三元，舊的三列檔案照讀。
  撤銷（退格刪光重選）、刪詞（`forget_word`）、減半都同時覆蓋二元與三元。
- **自動造詞**：使用者自己連著選出的兩個詞（不是整句路徑裡的），合起來不超過 4 個字、詞庫與使用者詞裡都沒有，
  且這條轉移已記夠次數（同一段拼音裡連著選的兩次，分兩段打的三次），就記成使用者詞並記一次選擇。
  同一段拼音裡自選一個詞之後剩下的部分走整句候選（`jidiaole` 選 擠、剩下 掉了 按空格）時，接縫處第一個詞的轉移按自選記雙份，
  但不參與兩詞造詞（我 + 的… 這種接縫太常見、轉移計數早就夠了，回放裡會把 我的 造成使用者詞，之後它就得和 沃德 按選擇次數比）。
  另外一段拼音分幾次選完（`CommitChain` 記著這段裡上屏的每個詞）時，合起來的文本按「整段字母 → 合成詞」記一次選擇（`user-choices.tsv`），
  記到兩次且詞庫裡沒有、不超過 4 個字就造成使用者詞，下次整段打出來它靠選擇次數直接排第一：這是「使用者手動拼了一遍整句」最直接的訊號，
  比等個人 n-gram 一份一份累到翻過靜態模型快得多（「擠掉了」靠 n-gram 要選四次）。
  詞級排序也用同一個語言模型：候選得分是 `log P(詞 | 上一個上屏的詞)`（個人 n-gram 插值，上文是鏈上的兩個詞）+ 封頂的選擇次數加分，詞庫詞頻只做預選和兜底，
  所以 `ba` 在「做了」後面出 吧、句首看模型；同一輸入串下選過的詞（`user-choices.tsv`，鍵是候選覆蓋的那段字母）排在得分之前，
  `mgs` 選過 美國式 下次就是首選；切分裡非末尾簡拼少的優先（`kaifa` 按 `kai fa` 讀的 開放 壓過按 `kai f a` 讀的 開放啊）。
  拼寫糾錯（Core `correction`）兩路：整段一處編輯的變體按噪聲通道挑（糾正後整句得分扣編輯代價仍高於原樣才糾），候選按糾正後的拼音出，
  消耗長度按編輯換算回原串；詞圖裡的敲錯邊（`correction::typo`，每個完整音節的一處敲錯變體當帶代價的位置寫法，與模糊音同一套 `Expanded`
  多寫法機制，`SpanWord::penalty` 進路徑得分，`Conversion::penalty` 讓 Engine 知道路徑不是原樣讀的）管「音節都合法、整句不通」的輸入，
  只進整句詞圖不進詞級候選。接受的糾正按原輸入串記選擇，回車原樣上屏的串記 `<raw>` 以後不糾；兩路接受的 (敲的, 要的) 音節對都記進個人敲錯表
  （`Learner::record_typo`，`user-typos.tsv`），那條邊與整段編輯的代價按次數打折（`TypoCosts::discounted`）。判斷帶快取（按作用域），commit / take_raw 複用 query 的結果。
  候選音節對回敲的字母（消耗、記敲錯）用 `Engine::align`，模糊音命中也走它。
  退格撤銷（`LastCommit`，Engine 留最近 4 次上屏 `recent_commits`）：上屏後殼把組句外的退格告訴 Engine（`note_backspace`），退格從最近一次往前數，
  一次上屏的字刪光了就候著；接著重打其中一段拼音（或其字首）選了別的詞，就把那次記的選擇次數、輸入串選擇、詞轉移、整段合成詞的選擇全部退回（`Learner::unrecord*`）。
  刪掉「沃德 書」兩個詞重打成「我的 書」也認得出（日誌裡這種錯法一天十幾次，以前只看最後一次上屏，一次都沒撤回，錯詞越選越靠前）；
  重打後選的還是同一個詞只把記錄丟掉；重打的拼音誰都對不上就當在改別處，全忘掉；刪得比記著的幾次加起來還多也全忘掉。
  標點、英文詞、原樣上屏這些沒學習的上屏也留一條只有長度的記錄，退格數過它們才能數到更早的詞。
  整句候選與某個詞候選文本相同時（使用者詞、或按別的讀音對上的詞）不重複插，但把那個詞提到整句該在的位置：整句轉換認定的最好讀法不該被詞級排序（別的詞選過更多次）壓在後面。
  雲端詞學成使用者詞時讀音以使用者敲的拼音為準（能切成與字數相同的完整音節、每個又是那個字的讀音時），否則用模型給的但逐字核對詞庫讀音，核不過不學：
  模型把「我的」讀音給成 `wo di` 之後，這條使用者詞按錯讀音出現在候選裡，把正確的整句候選「我的」頂掉、自己又排在「沃德」後面，就是這麼來的。
  使用者點選的轉移記雙份（`EXPLICIT_TRANSITION_WEIGHT`），整句路徑裡順帶的記一份：整句是模型自己算的，按空格接受會把它喂回模型形成回聲，
  使用者明確改選一次就要能壓過去。選擇次數在整句路徑上的加分取對數並封頂（`viterbi::WEIGHT_CAP`），只管同音詞偏好，不許它抬起拆分路徑。
- 聯想通過 `Predictor` trait 注入，與翻譯一樣是非同步補充：**不阻塞候選、不重排已有候選**，超時即丟。
  網路實現放 `qingjian-predict`，Core 不依賴它，也永遠不聯網。
  請求帶 `reasoning_effort`（配置 `[predict] reasoning_effort`，預設 `none`）：DeepSeek V4 這類預設思考的模型不關會把 token 預算花光、正文為空；
  金鑰環境變數預設 `QINGJIAN_API_KEY`（`api_key_env` 可改），名字跟產品不跟供應商，因為 `base_url` 本來就可以指到別家。
  介面是非阻塞的 `submit` / `poll`：網路 crate 自己開後臺執行緒做防抖、快取、超時；殼用定時器輪詢結果；
  Engine 給請求編號，只認最新序號的結果。觀察視窗在 Core 裡裁剪，殼給再多也只發這麼多。
- 只在組句中聯想（拼音 ≥ 2 個字母、停鍵 300 ms 後），一次請求兩種產物：
  **雲端詞**：使用者最可能想打的詞，帶正確全拼；模型可以糾錯（`zhgdoima` → 這個東西嗎），所以 Core 用容錯校驗
  （`prediction/fuzzy.rs`：字母與全拼的編輯距離，簡拼不算錯，容錯數隨長度增長，不到 4 個字母不容錯）而不是逐音節精確匹配；
  通過的插到候選第二位起（本地首選不動、其餘相對順序不動），雲朵標記，數字鍵選，上屏吃掉整段拼音；
  詞庫裡沒有的記成**使用者詞**（`user-words.tsv`，與主詞庫同格式），下次本地直接出且靠 weight 排前。
- **個人英文詞**（`Learner::learn_english` / `user_english`，`user-english.tsv`）：回車 / 英文模式直通原樣上屏的、像英文詞的字母串
  （中文模式下要求切不成完整拼音）和選中的英文候選，組成一張小 `WordList`，與隨包英文詞表一起出英文候選、排在前面。見 candidate-ui.md。
  **整句補全**：以這個詞開頭的完整說法（`suoyiwoxiangq` → 所以我想去吃飯），畫在 preedit 右側，Tab 接受。
- 上下文只來自應用（IMK `attributedSubstringFromRange:`）；終端、微信這類給不出的就只靠拼音。
  **本地輸入歷史不當上下文**：它是跨應用拼起來的碎片，用它聯想出來的全是噪音。
  上屏之後不聯想：沒有拼音約束的下文聯想每次上屏多發一次請求，純靠猜，已刪除。
- 聯想不限語言：模型按游標附近文本的語言續寫。
- **問字模式**（`?` 開頭）複用同一條通道：`PredictionRequest.kind = Question`，只帶問題拼音（不帶應用上下文、不要整句），
  `qingjian-predict` 按 kind 換系統提示，回覆是 `answers`（字 / 短答案 + 帶聲調讀音，放 `CloudWord.reading`）；
  Core 在問字模式下不做拼音校驗、本地不出候選。
- 發往雲端的上下文預設關閉；開啟後 Secure Input 絕不傳送，前後觀察長度可配置。
  上下文優先從應用讀（IMK `attributedSubstringFromRange:`），讀不到退回 Core 的本地輸入歷史（記憶體環形，可清除）。
- 個人化優先用線上 n-gram，神經模型只做重排與離線聯想，且要過評測門檻（見 roadmap Phase 7）。
- 兩者都依賴本地輸入歷史，歷史必須可檢視、可清除。

## 平臺層的技術決定

### macOS：IMK

- 使用 `objc2` + `objc2-input-method-kit`。
- 候選視窗自定義 NSPanel（豎排 / 橫排可配），不用 IMKCandidates。定位取游標所在那塊螢幕的可見區域：貼游標行下方，放不下放上方，左右不出屏；
  應用給不出游標矩形時以滑鼠位置為準。
- `apps/macos/src` 按職責分目錄，模組檔案只做 `mod` 宣告與 re-export：
  `main.rs` 初始化 host、建 IMKServer 並跑 NSApplication；
  `host/` 是程序級單例（一個 Engine + 一個候選視窗，`thread_local`，IMK 回撥全在主執行緒；`mod.rs` 放結構體與 `with`，`init.rs` 啟動載入、`config.rs` 熱載入、`settings.rs` 選單 / 偏好設定動作、`dictionaries.rs` 詞庫管理、`cloud.rs` 雲端、`diagnostics.rs` 診斷與日誌、`presenting.rs` 呈現），
  `host/` 下是會話狀態 `session.rs`、聯想輪詢定時器 `predict_monitor.rs`、配置檔案監視與定時落盤 `config_watch.rs`、
  短提示 `notice.rs`、翻譯選中文字的任務 `translation_job.rs`、附加詞庫裝配 `extra_dictionaries.rs` / `dictionary_info.rs`；
  `imk/`：`controller/`（`mod.rs` 是類定義與按鍵分發，`text` / `command` / `translate` / `display` / `commit` 各管一段）用 `define_class!` 繼承 `IMKInputController`（類名 `QingjianInputController`，
  與 Info.plist 的 `InputMethodServerControllerClass` 一致），只做按鍵 → Engine、Engine → 視窗；
  `client.rs` 用 `msg_send!` 封裝 IMKTextInput（`setMarkedText:` / `insertText:` /
  `attributesForCharacterIndex:lineHeightRectangle:` 取游標矩形）；`modifiers.rs` / `secure_input.rs` 查系統狀態；
  `candidates/`：`window.rs` 是非啟用浮動 NSPanel（level 101、CanJoinAllSpaces；用青簡渲染器時收滑鼠，點候選 / 譯詞上屏，
  `view/click.rs` 按渲染器給的點選區域認目標，`imk/controller/click.rs` 用 `activateServer:` 記下的控制器上屏，系統繪製時忽略滑鼠），
  `view.rs` 自繪頂部拼音行與候選（豎排 / 橫排兩套畫法），`theme.rs` 集中字型顏色間距，`row.rs` 把 Candidate 轉成展示片段，
  `preedit/`（`mod.rs` / `segment.rs` / `style.rs`）是拼音行的分段模型（由 Core 的 `MarkedSegment` 轉來），`frame.rs` 是一幀的資料；
  `menubar/`：`indicator.rs` 是選單欄的中 / 英 NSStatusItem（輸入源圖示沒法動態換，只能自己放一個），
  `menu.rs` / `action.rs` / `target.rs` 是輸入法選單；
  `preferences/`：偏好設定視窗（`window.rs` 手排控制元件、`layout.rs` 逐頁排版、`panel.rs` 關窗時切回啟用策略、`setting/`（`Setting` 與 `SettingValue`）控制元件 ↔ 配置項、
  `target.rs` 一個 `changed:` 選擇器、`key_recorder.rs` 快捷鍵錄製按鈕、`usage_page.rs` 「統計」頁（數字格子與「幾本《某書》」文案）、`about.rs` 「關於」頁文案、`edit_menu.rs` 只有編輯項的主選單、`file_dialog.rs` 匯入詞庫的打開面板）；
  `app/`：`paths.rs` 定位 `.app/Contents/Resources/`（詞庫、隨包領域詞庫 `dicts/`）與 `~/Library/Application Support/Qingjian/`（使用者資料），
  `settings.rs` 是配置檔案的執行時狀態，`logging/` 只寫 `~/Library/Logs/Qingjian/`（自己的 `LogFile` 按天分檔案、留 7 天、被刪重建），`bundle.rs` 讀 Info.plist，
  `input_source.rs` 是 `qingjian-macos --register`：走 Carbon TIS（`TISRegisterInputSource` + `TISEnableInputSource`，再起子程序 `--finish-register` 回讀 `IsEnabled` 並 `TISSelectInputSource`，隔 3 秒二次確認）把 `.app` 註冊成輸入源並切成當前。兩個坑：TIS 狀態按程序快取，本程序回讀永遠是舊值，只有新程序看得到；剛換過包的 3–5 秒內系統重掃會把剛啟用的記錄頂掉，所以要二次確認並啟用。
- **打包與分發**（`apps/macos/scripts/bundle.sh`）：版本號來自 workspace `Cargo.toml`，構建號是提交數，打包時用 PlistBuddy 寫進 Info.plist。
  `--install` 裝到 `~/Library/Input Methods/`（開發用）；`--pkg` 做 `target/pkg/qingjian-<版本>-macos-<arch>.pkg`：`pkgbuild` 元件包裝到
  `/Library/Input Methods/`（macOS 輸入法的慣例位置，需要管理員密碼；元件描述裡關掉 bundle 重定位，否則會裝到機器上同 id 的舊副本那裡），
  postinstall 殺舊程序並 `launchctl asuser <uid> sudo -u <登入使用者> qingjian-macos --register`（安裝器是 root，輸入源是每使用者的），
  `productbuild` 套上歡迎頁 / 許可證（`LICENSE`）/ 結束頁（`apps/macos/pkg/`）。簽名與公證全由環境變數決定：
  `QINGJIAN_SIGN_IDENTITY`（Developer ID Application，開 hardened runtime）、`QINGJIAN_INSTALLER_IDENTITY`（Developer ID Installer）、
  `QINGJIAN_NOTARY_PROFILE`（notarytool keychain profile，設了就公證並 staple）；沒設就 ad-hoc 籤 `.app`、pkg 不籤，
  測試者要在「隱私與安全性」裡點「仍要開啟」。解除安裝指令碼 `uninstall.sh` 隨包放在 Resources。二進位制只有本機架構，Intel 要另打。
- 配置只有一條通路：`Host::apply_config` 把當前 `Config` 推給 Engine（模糊音、模式鍵、Predictor 重建、釋義表切換）與介面
  （每頁候選數、翻頁鍵、外觀、☁︎ 標識、選單勾選、設定視窗控制元件）。啟動、選單開關、設定視窗、`host/config/watch.rs`
  每秒一次的 mtime 監視全都走它；三個入口都只寫 `config.toml`，不各存一套狀態。解析失敗沿用上一份，錯誤顯示在選單與設定窗口裡。
  按鍵走 `inputText:client:` +
  `didCommandBySelector:client:`，不用 `handleEvent:`。**組句期間 `didCommandBySelector:` 對不認識的
  選擇器也要返回 YES**：返回 NO 會讓應用自己處理方向鍵，應用一動游標就把 marked text 丟了，
  而我們的緩衝區和候選框還在（2026-09-03 踩過）。
- `define_class!` 的類在首次呼叫 `class()` 時才註冊到 ObjC 執行時，而 IMKServer 初始化時就按
  Info.plist 的類名查詢，找不到會**靜默退回基類**，症狀是按鍵全部透傳、像在打英文。
  必須先 `QingjianInputController::class()` 再建 IMKServer（2026-09-03 踩過）。
- `define_class!` 裡返回 `bool` 的方法體內不能 `return`（宏會把返回型別換成 ObjC `BOOL`），
  邏輯放到 inherent impl 裡，宏內只做轉發。
- Info.plist 約定：bundle id 是 `app.qingjian.inputmethod`（域名 qingjian.app 的反寫 + 產品，其他平臺外殼共用 `app.qingjian.` 字首），`TISInputSourceID` 與它相同，`InputMethodConnectionName` 必須是 `<bundle id>_Connection`；
  `LSBackgroundOnly = true`；ad-hoc `codesign` 之後 Apple Silicon 才會載入。
  **輸入模式**：`ComponentInputModeDict` 裡宣告單模式 `app.qingjian.inputmethod.Hans`（TextInputSources.h 規定的位置），
  系統登記、啟用、切換的都是模式，頂層 ID 只是父項，所以 `--register` 啟用的是 `tsVisibleInputModeOrderedArrayKey` 的第一項；
  模式顯示名在 `InfoPlist.strings` 按模式 ID 給，缺了對話方塊裡顯示裸 ID。沒有模式時標準文本檢視（備忘錄等）切不過去、
  「新增輸入法」列表也不出現（#31）。
  **圖示**：頂層 `tsInputMethodIconFileKey` 與模式裡的 Menu / AlternateMenu / Palette 三個圖示鍵都指向同一張 22×16pt 模板 PDF
  （黑色鍵帽鏤空圖形，`TISIconIsTemplate` 讓系統只取 alpha 按深淺色反色），鼠須管、Fcitx5 同此尺寸與形式，小了整體偏小、
  非模式路徑會被非等比壓進 16×16。系統自帶輸入法下拉選單裡的「拼」「あ」是蘋果私有素材（KeyboardLayouts.framework），
  `TISIconLabels` 第三方寫了不生效（鼠須管 #776 自 2023 掛著；goliajp/inputx、nvalleo/nagi 各自真機驗過），別再試。
  改圖示後系統有快取：`kill -9` TextInputMenuAgent / TextInputSwitcher，仍舊就登出。
- 輸入源註冊（`app/input_source.rs`，`--register`）：pkg 的 postinstall 以 root 跑，而輸入源是每個使用者的設定，
  所以 postinstall 切到登入使用者來調它。兩個坑決定了它的結構：剛換過 bundle 的頭幾秒系統還在重掃新包，這時啟用的記錄會被頂掉
  （實測裝完 3 秒內都這樣），所以啟用後隔一會兒要再確認一次；TIS 在程序內快取輸入源狀態，本程序怎麼重列表、跑 run loop
  回讀都是舊值，所以回讀與切換放在子程序（`--finish-register`）裡做。
- IMK 無法通過 `cargo run` 驗證：需要打包成 `.app`、裝到 `~/Library/Input Methods/`、
  登出或重啟輸入法程序才會生效。Core 的驗證靠 CLI 測試工具和單元測試，不依賴跑起真實輸入法。
- 已知需要單獨處理的場景：Secure Input 欄位、沙盒應用、Electron 與 Terminal 各自的 marked text 行為。

### Windows：TSF

- TSF DLL 會被載入進每一個應用程序，核心邏輯必須放在程序外。
  採用 Weasel（WeaselServer）和水杉（Server 程序）相同的結構：DLL 只做 IPC，Rust Core 跑在獨立程序裡。
- 使用 `windows` crate 的 COM `implement` 宏。
- TSF 是公認最難的輸入法 API，工時預期要按整個專案一半來估。

**已落地（骨架）：**

- **IPC 協議**：`qingjian-platform::protocol`，Server ↔ DLL 兩端共用、全部 serde。`ClientMessage`（DLL → Server：
  開 / 關會話、按鍵、上屏、回上下文、回選區、報中英模式）與 `ServerMessage`（Server → DLL：按鍵結果、上屏結果、非同步重繪、請求上下文、請求選區）；
  失焦 / 停用時 DLL 發 `Commit`，Server 回 `Committed { text }`（緩衝區原樣交出，對應 macOS 的 `commitComposition`），
  DLL 用最近收鍵記下的 `ITfContext` 經編輯會話落進文件；應用強行終止組句（`OnCompositionTerminated`）時拼音已被框架定成普通文本，
  DLL 只記「Server 緩衝過期」，下次說話前先 `Commit` 並丟掉交出的文本，不再插一次。
  中英模式：**Windows 與 macOS 機制不同**。macOS 用 Caps Lock 當中英切換鍵；Windows 按本地習慣，單擊切換鍵在中 / 英間翻轉，
  切換鍵由 `[shortcut] switch_mode` 勾選（`SwitchKeys`：單擊 `shift`（預設）/ 單擊 `control` / `ctrl+alt+space`，可多選，空列表 = 不用鍵切），
  `[general] english_mode` 關掉則整個內建英文模式停用（issue #81）。**模式全域性一份、存在 Server**（`Router.english`，與搜狗一致）：
  DLL 裡使用者切了（切換鍵、語言欄按鈕、右鍵選單、工作列轉換模式）用 `ModeChanged` 報上去；啟用、執行緒得到焦點（`com/focus.rs` 的
  `ITfThreadFocusSink`，切視窗時 `ITfKeyEventSink::OnSetFocus` 不觸發）和每隔幾拍的輪詢用 `SyncMode` 取回並跟上
  （`service/mode.rs::adopt_mode`，不回報）；懸浮狀態條上點「中 / 英」直接改 Server 那份。有 DLL 來取模式也就說明青簡是當前輸入法，
  狀態條據此顯示，`ImeSwitched` 收起。之前模式各應用各記一份、Server 只採納「前臺會話」的上報，後臺執行緒啟用、新應用繼承都會讓
  各處對不上，改成全域性後這些都不存在了。
  單擊判定在**擊鍵 sink** 裡（`com/key/tap.rs`，喂 `OnTestKeyDown` / `OnTestKeyUp`：按下切換鍵到抬起之間沒有別的鍵插進來就是一次單擊；
  微軟 SampleIME 的 `OnTestKeyDown` 同樣處理 VK_SHIFT，sink 收得到獨立修飾鍵）。之前用執行緒級 `WH_KEYBOARD` 鉤子判定，但鉤子**看不到被 TSF 吃掉的鍵**
  （msctf 在佇列層把它們改成 WM_NULL），Shift + 數字（刪候選 / 第二譯詞）會被誤判成單擊而切換模式，2026-09-11 真機確認 sink 收得到 Shift 後鉤子已刪。
  切換鍵、英文模式開關與「Shift 字母進組句」由 **Server 讀配置、經協議下發**（`protocol::InputSettings`：`OpenSession` 的回包
  `SessionOpened` 帶一次，之後每拍 `SyncMode` 跟著走，值變了就地應用，**設定視窗改完約 320 ms 內生效**，不必切走再切回輸入法）；
  DLL 不讀配置檔案——它跑在每個應用程序裡，AppContainer 裡的商店應用連 `%APPDATA%` 都讀不到。
  `ctrl+alt+space` 是組合鍵、屬系統鍵不經擊鍵 sink，與翻譯快捷鍵一樣登記成 TSF 保留鍵（`com/key/preserved.rs` 的 `GUID_SWITCH_MODE`）。
  勾選項裡沒有 Ctrl + Space：中文 Windows 把它綁成系統的「輸入法/非輸入法切換」（`IME_CHOTKEY_IME_NONIME_TOGGLE`），系統先截走、保留鍵收不到。
  但我們**適配**這條系統熱鍵（與微軟拼音一致，不進勾選項）：它翻的是「輸入法開 / 關」compartment（`GUID_COMPARTMENT_KEYBOARD_OPENCLOSE`），
  `com/mode/sink.rs` 監聽它（`sync_from_keyboard_open`）：關 = 英文、開 = 中文，照樣報給 Server 成為全域性模式。反向由 `refresh_mode_indicator`
  每次把開關寫成與模式一致（中文開、英文關；新執行緒裡它預設是關），系統熱鍵下一次按下才總是真的切換、不白按；關著時按鍵照樣送到 TIP（真機日誌驗過）。
  它的 Space 被系統截走，擊鍵 sink 只看到 Ctrl 按下又抬起，會被當成單擊 Ctrl，所以開關一變就作廢正按著的單擊（`KeyTap::cancel`）。
  裝了別的鍵盤佈局 / 輸入法時 Windows 可能改用這條熱鍵換輸入法，那由系統決定。
  **四條切換入口**——單擊切換鍵、語言欄按鈕、懸浮狀態條、工作列轉換模式 compartment——都匯到 `com/service/mode.rs::set_english_mode`，
  內建英文模式關著時在那裡一併攔住（Server 側狀態條點選按同一項攔，免得兩邊顯示不一致），此時連語言欄的「中 / 英」按鈕都不登記。
  「翻譯選中文字」的快捷鍵（預設 Ctrl+Alt+T）**登記成 TSF 保留鍵**（`com/key/preserved.rs`，`ITfKeystrokeMgr::PreserveKey` → `OnPreservedKey`）：
  帶 Alt 的組合是系統鍵、不經擊鍵 sink（真機 `OnTestKeyDown` 裡從沒出現過），保留鍵由 TSF 在應用之前匹配，UWP 裡也一樣；組合啟用時從
  `config.toml` 讀一次（AppContainer 讀不到使用者目錄時用預設），命中後當作那個組合鍵轉發給 Server 走原有的 `RequestSelection` 流程。
  DLL 記 `english_mode` 持久狀態；工作列的中 / 英指示器靠 `GUID_LBI_INPUTMODE` 語言欄按鈕渲染
  （`com/mode/button.rs`，圖示是設計稿 SVG 預柵格化的四檔 alpha 蒙版，`mode/icon.rs` 按系統 DPI 挑檔、按工作列 `SystemUsesLightTheme` 填黑或白，Caps Lock 亮著顯示「A」；第三方 TIP 單寫轉換模式 compartment 不出這個指示器），另外順帶寫一份轉換模式 compartment
  （`GUID_COMPARTMENT_KEYBOARD_INPUTMODE_CONVERSION` 的 `TF_CONVERSIONMODE_NATIVE` 位，`com/mode/mod.rs`）。這條 compartment 還**反向同步**：啟用時對它掛
  `ITfCompartmentEventSink`（`com/mode/sink.rs`，與「輸入法開 / 關」共用一個 sink），使用者點工作列中 / 英（或別的輸入指示器途徑）改了轉換模式時 `OnChange` 讀回 `NATIVE` 位、與當前
  `english_mode` 不同才翻轉（相同即我們自己寫的那次，忽略以防迴環），翻轉只刷語言欄按鈕與懸浮狀態條、**不回寫 compartment**
  （`follow_system_mode`：在它自己的 `OnChange` 裡寫它會被拒、報 0x8000FFFF，要寫的值也本來就等於當前值）；
  **啟用後的最初一瞬除外**（`service/mode.rs` 的 `CONVERSION_RESTORE_GUARD`，約 1.2 s）：msctf 會在 TIP 啟用後 200–300 ms 把 profile 存的
  轉換模式寫回 compartment，那不是使用者操作，採納了會被當成使用者切換、改掉全域性模式。Caps Lock 只管大小寫，
  亮著無論中英模式都直接出大寫英文（微軟拼音式）。`KeyModifiers` 因此帶 `caps`（大小寫）與 `english_mode`（持久模式）兩個非物理位，
  字母大小寫按 `shift XOR caps`。Router（`dispatch/key/input.rs`）裡 `english = caps || english_mode`、候選只在 `english_mode && !caps && 應用允許` 時給，
  `[general] english_candidates` 關著就是純直通。macOS 的「先上屏、再把這個鍵交給應用」在 Windows 上會亂序
  （放行是同步的、上屏走非同步編輯會話），所以組句中的空格 / 標點改成吃掉、連同上屏文本一起插入；Shift 大寫字母交 [`Engine::push`] 進緩衝區
  （Core 按小寫匹配、原樣上屏時還原大寫，見 [`Composition`]），不再走「先上屏再放行」；
  組句外沒有全形對映的字元裡，`-` `=` 也改由殼自己插入（`dispatch/key/input.rs::apply_punctuation`）：
  放行要等宿主把鍵交回應用，實測在部分宿主（Edge / QQ 等）裡這個鍵到不了，使用者看到的是「按了沒反應」；
  其餘（`@` 數字等）繼續放行，宿主連不上 Server 時也只吃「可能是在打拼音」的字母（`service/key_sink.rs`），
  免得斷連窗口裡標點 / 數字跟著一起沒反應；
  `[apps] english_candidates_on` 白名單按應用給（`english_candidates_off` 從中排除）：應用標識在 Windows 上是宿主程序的 exe 檔名（DLL 載入在應用程序裡，`GetModuleFileNameW(NULL)`
  取到就隨 `OpenSession { app }` 報一次，Server 每會話記下，收鍵時按當前會話查），預設名單分平臺（`AppsConfig` 的三份常量與配置模板的 `[apps]` 一節按目標平臺三選一：Windows 是 exe 檔名、macOS 是 bundle identifier、Linux 是 Fcitx5 認到的應用名，X11 下是 WM_CLASS、Wayland 下是 app_id），
  經典控制台的視窗屬於 `conhost.exe`、Windows Terminal 是 `WindowsTerminal.exe`；
  Server 把按會話應用算好的「給不給英文候選」隨 `InputSettings` 下發，不給時英文模式與 Caps 下沒在組句的字母 DLL 在 `OnTestKeyDown` 就不吃、原樣交給應用
  （吃了再由輸入法插入的話，Photoshop 畫布這類沒有文本框的地方收不到 W / H / V 單鍵快捷鍵）；
  `[shortcut]` 的修飾鍵 + 數字（譯詞上屏 / 刪候選，`dispatch/key/shortcut.rs`）：配置裡的 `Modifiers` 按 option→Alt、control→Ctrl、command→Win
  落到 `KeyModifiers`，Router 按鍵碼認數字、去掉 Caps 位後與配置比；DLL 見 Ctrl / Alt / Win 仍一律放行，只有組句中的修飾鍵 + 數字送 Server 判，
  沒配到的 Router 回 Passthrough。刪候選的那句反饋（「已刪除…」/「沒什麼可刪」）隨下一幀的 `Frame::notice` 下發，自繪候選窗畫在拼音行下方、
  顯示到下一次按鍵（`dispatch/key/shortcut.rs` 填、`handle_key` 開頭清；對齊 macOS 畫在拼音行右側的短提示）。
  **翻譯選中文字**（`[shortcut] translate_selection`，預設 Ctrl+Alt+T）：不在組句、雲服務開著時按下它，Server 回 `RequestSelection`
  （替代那次按鍵的常規結果），DLL 起一個**非同步只讀編輯會話**（`com/edit/selection.rs`，`GetSelection` + `GetText` 讀選中文本、上限 500 字、`GetTextExt` 量螢幕矩形）
  回 `Selection { text, rect }`；Server 走 Core 的 `request_translation`（`PredictionKind::Translate`，雙向，譯文走 `sentence`），
  在**自繪候選窗**裡以選區矩形為錨顯示單條譯文（先「翻譯中…」，雲端回來再換），評審態吃走所有鍵：回車 / 空格接受、Esc 保留原文、其餘鍵放棄並交回應用。
  替換選區不另加協議——接受時 Server 把譯文當 `commit` 回給 DLL，DLL 無活動組句時 `InsertTextAtSelection` 正好替換當前選區。DLL 用 `Shared::translating` 標誌讓評審期吃鍵、輪詢定時器照常拉雲端譯文、失焦收窗。
  一次要繪製的狀態是 `Frame`（preedit 分段 + 候選頁 + 可選的整句補全 `sentence` 與刪候選提示 `notice`，後兩者不參與 `Frame::is_empty`），preedit 用 `PreeditSegment`（Core `MarkedSegment` 的可序列化映象，
  協議不耦合 Core 內部列舉），候選直接嵌 `qingjian_core::CandidateList`。同詞幹型別收進子目錄：`key/{event,outcome}`、`frame/preedit/{kind,segment}`。
- **Server 程序**：`apps/windows/server`（package `qingjian-windows-server`，bin `qingjian-server`）。`dispatch::Router` 按 `SessionId` 分派多會話（Windows 一個 Server 服務多個應用程序，
  每會話各持組句狀態，不同於 macOS 的程序級單例）。會話開 / 關、按鍵與上屏、Engine 裝配、命名管道傳輸（`\\.\pipe\qingjian`）都已跑通，Windows 上端到端測過。
- **候選視窗（Server 程序自繪 + uiAccess）**：候選窗從前在**應用程序內的 DLL** 自繪，普通置頂窗被微軟商店 / 工作列搜尋這些**更高 z-band** 的宿主蓋住。現改由 **Server 程序**自繪（`server/src/ui/`：一條專用 UI 執行緒註冊視窗類 + 建 GDI 分層窗 + 跑訊息迴圈，HWND 只在該執行緒碰；工人執行緒經 `Sender<UiCommand>` + `PostThreadMessageW(WM_APP)` 把「顯示(`Frame`+螢幕矩形) / 隱藏」marshal 過去；程序級 `SetProcessDpiAwarenessContext(PER_MONITOR_AWARE_V2)` 按物理畫素對齊應用報來的矩形）。DLL 只量游標螢幕矩形（`GetTextExt`，退滑鼠）發 `PositionCandidates{rect}`，並在組句於 DLL 側結束（應用終止組句 / 斷線，`OnCompositionTerminated` 這條 Server 無從知曉）時發 `HideCandidates`；Server 握著 `Frame` 直接自繪，雲端非同步更新也直接刷自己的窗、不回傳 DLL（渲染程式碼——詞性 + 譯文 + 分頁 + 柔和陰影，對齊 macOS——整塊從 DLL 搬到 Server）。**蓋過高 z-band 宿主**靠 Server exe 的 `uiAccess="true"` manifest（`server/build.rs` 用 embed-manifest 嵌）+ 程式碼簽名 + 裝 Program Files 三者齊備（`SetWindowPos(HWND_TOPMOST)` 才自動升進 UIAccess 高帶）：開發自籤 + 本機受信任根（`installer/sign-local.ps1`），發版換 Certum 開原始碼簽名證書；uiAccess exe 不能 CreateProcess 拉起（報 740），裝完 / 登入都走 ShellExecute（安裝器完成頁 `ShellExecAsOriginalUser` + `{commonstartup}` 啟動快捷方式由 Explorer 拉起才授 uiAccess，故不用計劃任務）。候選窗每顯示一頁，Server 調 `Engine::note_displayed`（收窗傳空）告知當前頁——生詞「看到輪次」據此推進、橙色標記滿 `FRESH_UNTIL` 輪才畢業，對齊 macOS 殼的 `render`。
  候選窗只在候選與譯詞的點選區域收滑鼠（`WM_NCHITTEST` 回 `HTCLIENT`，其餘 `HTTRANSPARENT` 穿透，`MA_NOACTIVATE` 不搶焦點，`ui/candidates/click.rs`）；
  點中的目標經工人通道交給 Router（`dispatch/key/click.rs`，與數字鍵 / 譯詞鍵同一套上屏），Server 寫不了文件，上屏的字掛在焦點會話上，
  DLL 下一拍 `Poll` 隨 `Update.commit` 取走、開編輯會話寫進去（協議 v8 起；更老的 DLL 不收點選），趕在輪詢前來的鍵把它接在結果前面。
- **懸浮狀態條（Server 程序自繪，可拖動 / 記位置）**：桌面上常駐的小浮窗，顯示當前中 / 英（開著雙拼時附方案名），與工作列的中 / 英指示器（語言欄按鈕）並存。跟候選窗**同一條 UI 執行緒**、複用同一套分層視窗合成器（`server/src/ui/layered/`：圓角背景 + 四周柔和陰影，從候選窗的 `surface.rs` 抽出來兩邊共用）與主題（字型 / 配色 / DPI / 深淺）；自己一個視窗類與視窗過程（`server/src/ui/status/`）：三格 `[中 / 英][，。/ ,.][⚙]`：按下滑鼠先 `DragDetect`，挪出閾值就交給系統移動迴圈（`WM_NCLBUTTONDOWN` + `HTCAPTION`，結束時 `WM_EXITSIZEMOVE` 報新位置），沒挪就是點選、按 x 落進哪格；`WM_MOUSEACTIVATE` 回 `MA_NOACTIVATE` 點它不搶應用焦點；視窗過程按 HWND 從 thread_local 表查到物件。點格 / 拖動結束經 `StatusEvent`（`dispatch/status/`）投回工人執行緒（工人迴圈收的是 `ipc::Work`：DLL 訊息或狀態條事件），Router 寫回配置（`[status_bar] x/y`、`[general] full_width_punctuation`，熱載入再讀回）；齒輪由 UI 執行緒直接起設定程式。中英模式全域性一份、存在 Server（見上文「中英模式」）：DLL 裡使用者切了用 `ModeChanged` 報來，狀態條上點「中 / 英」直接改 Server 那份，各 DLL 啟用、得到焦點時和輪詢定時器（沒組句、本執行緒前臺時每幾拍）用 `SyncMode` 取走跟上；有 DLL 來取模式就說明青簡是當前輸入法，狀態條據此顯示。會話號用執行緒 id（`com::session_id`）——TSF 的 client id 各程序都是同樣那幾個值，拿它當會話號會撞。前臺執行緒沒連著 Server 時（登入後 Server 起得比第一個應用晚、Server 重啟過）輪詢那一拍順帶補連，連上報一次當前模式（重啟過的 Server 不知道）。狀態條**常駐桌面**，只跟「當前輸入法是不是青簡」走：第一次 `ModeChanged` 顯示，DLL 掛 `ITfActiveLanguageProfileNotifySink`（`com/profile.rs`）在別的 TIP 被啟用時用一條臨時連線發 `ImeSwitched` 收起（此時自己已被停用、會話連線已關），應用退出（`CloseSession`）不收。雙拼方案 Server 從自己的 `[general] shuangpin` 配置知道，不必帶。開關與記住的位置在 `[status_bar]`（`enabled` / `x` / `y`），熱載入即時生效；uiAccess 高 z-band 與候選窗同進程天然繼承。參考微軟水杉的 FTB 形態（`~/Desktop/MSIME-Windows`，它用 D2D + DirectComposition 且不記位置），落地時選沿用本專案已有的 GDI 分層窗那套以保持視覺語言一致、並加了位置持久化。
- **幀編解碼**：長度字首 JSON 幀的 `read_message` / `write_message` 與預設管道名放在 `qingjian-platform::protocol`，Server 與 DLL 共用（DLL 不必依賴整個 Server 庫）。
- **TSF DLL**：`apps/windows/tsf`（package `qingjian-windows-tsf`，`cdylib`，產物 `qingjian_tsf.dll`，依賴官方 `windows` crate 的 COM `implement` 宏）。「引擎層」不是 Engine 而是連 Server 的**管道客戶端** `EngineClient`（平臺無關、可端到端測）；
  連不上 Server 時 DLL 自己把它拉起來（`com/service/launch.rs`）：Server 只在登入時由「啟動」資料夾的快捷方式拉起，中途掛了以前只能等下次登入、期間靜默吞鍵。
  用 `ShellExecuteW`（`uiAccess=true` 的 exe 用 `CreateProcess` 報 740），與 DLL 同目錄的 `qingjian-server.exe`；
  程序內 5 秒冷卻 + 跨程序命名互斥體（`Local\QingjianServerLaunch`）保證多個應用同時發現 Server 不在時只起一個。
  COM 層：`DllGetClassObject` → `IClassFactory` → `#[implement(ITfTextInputProcessor, ITfKeyEventSink, ITfDisplayAttributeProvider)]` → `Activate` 掛擊鍵 sink + 登記翻譯保留鍵 + 語言欄中英按鈕 + 連管道 → `OnKeyDown` 轉發按鍵、經非同步編輯會話（`TF_ES_READWRITE`，不帶 SYNC）寫組句 / 上屏；`DllRegisterServer` 寫 InprocServer32 並經 `ITfInputProcessorProfiles` / `ITfCategoryMgr` 註冊文本服務與各能力類別；語言 profile 在「中文(簡體)」與「中文(台灣)」下各登記一個（後者顯示為「青簡」、預設出繁體，DLL 查到當前 profile 後以 `ClientMessage::Profile` 報給 Server，Server 按會話記、與 `[general] traditional` 取或）。
  組句拼音的**內聯下劃線**（對應 macOS marked text 下劃線）走 TSF 顯示屬性協議（`com/display_attribute/`）：註冊 `GUID_TFCAT_DISPLAYATTRIBUTEPROVIDER` 類別 + 一個自定義顯示屬性 GUID（細實線、`TF_ATTR_INPUT`），
  `ITfDisplayAttributeProvider`（實現在 TextService 上）把 GUID 對應的 `TF_DISPLAYATTRIBUTE` 交給系統；收鍵寫組句時用 `ITfCategoryMgr::RegisterGUID` 把 GUID 換成 atom，`SetValue` 進組句範圍的 `GUID_PROP_ATTRIBUTE` 屬性，宿主據此在拼音底下畫線。
  收鍵與執行細節記進 `%LOCALAPPDATA%\Qingjian\logs\tsf.<日期>.log`（與 Server / 設定程式同目錄，按天一個檔案、留 7 天；多程序追加同一檔案）；候選視窗不再由 DLL 自繪（已搬到 Server 程序，見上「候選視窗」），DLL 側只做 preedit 內聯 + 上報游標矩形；雲聯想已接。
- **交叉編譯驗證**：`qingjian-core` / `-dictionary` / `-format` / `-lm` / `-platform` / `apps/windows/{server,tsf}` 已能
  `cargo check --target x86_64-pc-windows-gnu` 通過（藉此修掉 `qingjian-format` 裡 unix 專有的 `Mmap::advise` 未 `cfg` 的移植 bug）；
  本機只 `check`，真正編譯在 Windows 機器上做（`qingjian-neural` 的 candle 後端在 Windows 走 CPU，已接進 Server，見下「本地整句模型」）。
- **本地整句模型（Server 程序，與 macOS 的 `host/model/mod.rs` 對齊）**：`server/src/dispatch/rescore/`。啟動時 `find_model` 優先找使用者 `%APPDATA%\Qingjian\models\hanzhang-tongbian\` 與隨包 `data\models\hanzhang-tongbian\`，缺失時回退 `hanzhang-zhiwei`；舊使用者目錄仍可讀取。接受 `.qjm` 單檔案或三件套目錄；`[model] enabled` 開著就起執行緒載入並預熱（`ModelLoader`），下一次按鍵 / tick 接上 `set_async_sentence_scorer`。
  Server 沒有定時器：緩衝變化後 `schedule_rescoring` 起防抖，工人迴圈 `recv_timeout(router.next_tick())` 按 `RescoreState` 的節拍醒來（防抖 80 ms → `request_rescoring`；然後 20 ms 一次 `poll_rescoring`，最多等 2 s），DLL 組句期間每 80 ms 的 `Poll` 也順帶 `tick`。分到了重查一次、重建候選佈局（雲端詞與整句補全留著）、由 Server 自繪的候選窗直接重畫，DLL 下一次 `Poll` 拿到新幀更新內聯 preedit；翻過頁 / 動過高亮不動。熱載入 `[model]` 變了才過載 / 解除安裝。
  前文：DLL 在**起組句的那次讀寫編輯會話**裡順手讀選區起點前 64 個 UTF-16 單元（`com/edit/surrounding.rs::text_before_caret`，拼音還沒插進去、不用再開一次會話），隨 `ClientMessage::Surrounding` 單向送來。**密碼框與私密輸入**（2026-09-12 查了微軟文件 / SampleIME / Chromium 原始碼後定）：
  TSF 規定鍵盤類 TIP 必須看上下文的 `GUID_COMPARTMENT_KEYBOARD_DISABLED`（微軟文件明說密碼框應停用文本服務、`IS_PASSWORD` 只是標註不提供保護；Chromium 給密碼框的上下文設的就是它），
  DLL 在 `OnTestKeyDown` / `OnKeyDown` / 保留鍵裡沒在組句時先查它（連同 `EMPTYCONTEXT`，`com/context.rs`），非零整鍵放行、不組句——與 macOS 的 Secure Input 同一語義；
  輸入範圍（`GUID_PROP_INPUTSCOPE`）只在起組句那次編輯會話裡讀一次（`com/edit/surrounding.rs::input_context`）：含 `IS_PRIVATE` / 密碼 / PIN 之一算**私密**——Chromium 原始碼裡密碼框與不學習的輸入框對映成 `IS_PRIVATE`（含義「別學」；2026-09-12 box 實測 Edge InPrivate 的網頁文本框報的仍是 `IS_SEARCH`，`IS_PRIVATE` 只在密碼框見過，這條是兜底）——私密時不讀前文，並隨 `ClientMessage::Privacy` 告訴 Server（客戶端只在變了時發；記事本等不支援該屬性的應用 `GetValue` 失敗按不私密）。
  Server 按會話記 `private`、焦點切換時重設，Core `Engine::set_private`：學習器與輸入日誌外面各套一層 `Muted*`（寫吞掉、讀照常，排序不變），聯想 / 翻譯 / 釋義兜底不發。協議版本見 `qingjian-platform::protocol::PROTOCOL_VERSION`。
- **版本與釋出**：各平臺殼版本號獨立（見 `docs/notes/release.md`）；`apps/windows/server/Cargo.toml` 寫死自己的 `version`，
  釋出標籤見 `docs/notes/release.md`（0.1.4 起三個平臺共用 `v<版本>`）。`qingjian-windows-tsf` 是同一 Windows 產品的另一半（各自 `Cargo.toml` 記版本；兩個 package 同放 `apps/windows/` 下，是一個產品的兩個產物——不合成一個 crate，因為 DLL 不能帶 Engine 的依賴樹）。

### Linux：Fcitx5 / IBus（2026-09-15 定，PR #90 在做）

- 與 Windows 同構：Core 跑在獨立的 Rust Server 程序（Unix socket，複用 `qingjian-platform` 的協議），殼只轉發按鍵、上屏與游標位置。
  詞庫、語言模型、神經模型、渲染器和字型都在 Server 裡，不進 Fcitx5 這個所有輸入法共用的程序；Server 崩了不帶倒 Fcitx5。
- Fcitx5 殼是一層薄 C++ 外掛（addon ABI 是 C++ 虛類，純 Rust 做不了），只做轉發，判斷都在 Server。
- 候選窗自繪：Server 出點陣圖並自己開窗（X11 / XWayland 先行），外掛只報游標矩形與焦點。
- IBus 殼以後用 zbus 寫成純 Rust，共用同一個 Server；排在 Fcitx5 真機驗收之後。
- 三個平臺的按鍵分流（macOS host / Windows Server / Linux Server）要抽成公共 crate，等 Linux 最小輸入鏈路合入後做。
