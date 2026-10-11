# 各 crate 的實現要點

CLAUDE.md 只保留目錄地圖與規則，每個 crate / app / tool 的實現細節收在這裡：入口型別、資料檔案、常數、生成命令。
改了實現要同步改這裡；與程式碼衝突時以程式碼為準。

## crates/qingjian-dictionary

詞庫（TSV 解析或 `.qj` mmap），鍵按位元組序排好，查詢逐音節位置二分收窄（簡拼位置按音節塊跳掃），
`lookup_pattern`（≥ 模式長度）與 `lookup_exact`（正好等長）同一套實現。詞庫鍵以 `v` 表示 ü，
TSV 解析、查詢與生成工具把 `lue` / `nue` 統一成 `lve` / `nve`。
舊 `.qj` 含這些鍵時，載入器建立規範化的記憶體詞庫。新 `.qj` 繼續使用 mmap。
輔碼碼錶（`aux_code_table/`，`AuxCodeTable` = `Kind::AuxCodeTable` 的 `.qj`）是另一套儲存：`TEXT` 詞 arena + `CODE` 碼 arena +
`ENTR` 條目（12 位元組，詞位元組序升序、同詞相鄰）+ `HASH` 詞 → 條目區間起點（`qingjian_format::hash`）。查詢只有
`AuxCodeLookup::code_with_prefix` 一個方法：候選詞逐個問「有沒有以當前碼段開頭的碼」。匯入 `import_aux_code_table`
解析 Rime `.dict.yaml` 的 `columns`（預設 text / code / weight）與 `import_tables`（相對主檔案目錄合表），
「有詞無碼」與非法碼進統計返回、不靜默跳過；一個可用條目都沒有時報 `NoCodeEntries`。

`CodeTable` 是形碼碼錶（五筆），與詞庫並列的另一類查表：鍵是編碼本身（`ggll`），不是拼音音節序列，
格式 `詞\t編碼\t詞頻`（`assets/wubi/wubi86.tsv`，8.9 萬條），按 `(編碼, 詞頻降序)` 排好、`lookup` 二分定位字首區間。
編碼打全的詞（`exact`）排在同字首的更長編碼詞前面，這就是一級 / 二級簡碼的取法，不需要另做簡碼錶。
與詞庫的一處關鍵差別：碼錶沒有 `.qj` 容器，只讀 TSV。

## crates/qingjian-core

模組：`composition`（緩衝區與游標；中文模式下 Shift+字母按小寫進 `buffer` 參與匹配、大寫記在 `shifted`，`typed_text` 還原後用於原樣上屏）/ `parser` / `correction`（拼寫糾錯：整段一處編輯的候選糾正 + `typo` 音節級敲錯變體表，後者進整句詞圖當帶代價的邊）/
`candidate` / `ranking` / `shortcut` / `sentence` / `fuzzy` / `shuangpin`（雙拼：七套方案鍵位表、鍵 → 全拼解碼與消耗換算）/ `zhuyin`（大千注音：鍵 → 注音符號 → 拼音，`[general] zhuyin` 開關，聲調只判音節完整不進查詢）/ `emoji` /
`english`（英文模式候選）/ `engine`（`query::EnglishTail`：句末英文詞併入整句，`woxiangxuehaorust` → 我想學好rust，尾段也像拼音時按分數與拼音讀法比）。
輔碼（`engine/aux_code.rs`）：`Engine.aux_code: Option<String>` 是碼段（`None` 拼音態，`Some("")` 剛觸發或刪空停在輔碼態——`Engine.aux_keep_empty`，配置 `[general] aux_code_keep_empty` 預設開），
不進 `Composition`；`aux_trigger`（配的觸發鍵 + 游標在段尾 + 作用域能完整切分 + 雙拼韻母鍵優先）、`enter_aux`、
`push_aux_code`（只收 a-z）、`clear_aux`；退格在輔碼態內部分派（刪碼；刪空按 `aux_keep_empty` 停在輔碼態或回拼音態，空碼段再退格退出），`commit_with` / `take_raw` /
`punctuate` / `clear` 都收尾清碼。查詢在 `rank` 之後**反向過濾**：候選詞逐個 `AuxCodeLookup::code_with_prefix`
（碼錶在 `Engine.aux_codes`，`set_aux_codes` 注入），不命中的隱藏，命中的按「完全匹配碼 > 碼長降序 > 原詞頻序」
重排（stable sort 保住原序），命中碼（沒在篩碼時是詞的首條碼）寫進 `Candidate.aux_code`；碼段非空時跳過整句 / 英文 / 快捷 / emoji / 自定義短語
與雲聯想。preedit 分段多出 [觸發鍵 `Typed`][碼段 `MarkedKind::AuxCode`]，見 `Query::marked_segments`。

雙拼 `Scheme::syllable` 優先解出鍵位表裡的合法音節；解不出且該韻母鍵含 `v` 時，j / q / x / y + ü 規範化為 ju / qu / xu / yu。只補此前無效的組合，不覆蓋微軟 `jv` → jue 等既有解碼。

形碼（五筆）在 `engine::query::code`。`Engine` 上有兩個開關：`set_code_table`（碼錶）與 `set_phonetic`（拼音側參不參與），
在 `query_inner` 進切分之前按這兩個分派——只有拼音 / 只有形碼（`query_code`）/ **兩邊都開（`query_mixed`，混輸）**。
編碼按字首查表，`CandidateKind::Code` 的候選 `syllables` 為空、上屏吃掉整段作用域（`whole_scope`）。
混輸是「拼音那條 Query 前面插上形碼候選」：拼音讀不出來時（`ggll`）整個按形碼走，兩邊都空才算錯；
按文本去重，編碼打全的形碼詞在前、拼音居中、只命中字首的形碼詞墊後（一律形碼在前的話 `kai` 的首選會變成編碼 `kaik` 的詞）；混輸下模式鍵同雙拼換成大寫，四碼以內不做拼寫糾錯，且五筆碼最長 4 位、第 5 個字母起自然只剩拼音。
拼音那套在純形碼下全部不適用，靠 `modes()` 返回 `ModeKeys::LETTERLESS` 與 `active_correction` 直接返回 `None` 關掉；
譯詞標註、生詞記錄、輸入日誌、使用者選擇學習與個人 n-gram 仍照常工作。
`Engine` 是對外唯一門面，`Translator` / `Learner` trait 在 `engine` 模組；詞庫是「主詞庫 + 附加詞庫（`set_extra_dictionaries`）+ 使用者詞」的列表；繁體輸出（`traditional` 開關與 `traditional_map` 對映）依賴 `ferrous-opencc`（`s2tw`）在出候選與上屏邊界轉換，內部保持簡體。
- 中英混輸的英文詞位置：`Engine::set_chinese_first`（配置 `[general] chinese_first`，預設關）關著時拼音不像話的輸入英文排第一（`extras::insert_english`，
  使用者老選中文詞時仍讓中文在前），開著時整句先插、英文詞緊隨其後排第二（`query_inner` 裡兩步的先後按開關掉轉）；句末英文詞併入整句（`EnglishTail`）不受它影響。
  預設關是回放定的（9241 詞 / 269 條英文上屏：預設開英文首選 82.5% → 7.1%）。
- `custom_phrase::merge_replacements` 把平臺給的「輸入碼 → 短語」表（macOS 系統文本替換）並進配置裡的自定義短語：每條佔該碼最靠前的空位（1–9），
  輸入碼不是小寫字母、已有同碼同文本、九位都滿的跳過；Core 不管資料從哪來。
- 拼寫糾錯（`correction`）：整段一處編輯的變體裡相鄰換位允許末尾音節沒敲完（`loose_segmentation`，`mignt` → `ming t…`；
  有「每個音節都完整、末尾不是落單單字母」的變體時殘尾變體不參與，免得 `keyyi` 的 可以 被 `ke yi y…` 搶走），
  糾正之間比較時換位減 `TypoCosts::correction_transpose_discount`（1 nat，CLI `--tune correction-transpose=`），與原樣比不減；
  2026-09-15 回放 283 詞 / 23 句：詞首選 89.0%、整句 82.6%，與改前持平（折扣若也用在與原樣比，`zhongwne` 會誤糾成 中文，整句掉一條）。

`EngineSession` 儲存可掛起的組句、標點、歷史與學習鏈，`Engine::swap_session` 在同一個引擎裡交換輸入狀態，共用詞庫與落盤服務。切換上下文時清除查詢及非同步預測快取，並由平臺恢復各自私密狀態。

`Engine::raw_preedit()`（`engine/raw/`）只讀返回 `RawPreedit { text, cursor_bytes }`：完整未上屏組合及 UTF-8 位元組游標，與隨後 `take_raw()` 共用文本生成，保留大小寫、顯式分隔符及游標後的剩餘內容，不包含待處理輔碼；不執行候選查詢、不學習、不記日誌、統計、歷史或展示回報。
注音繼續輸出符號；游標按解碼單元內按鍵字首產生的符號數對映到完整單元的同序字元邊界，輕聲先敲時採用邏輯位置，一聲空格不佔顯示字元。未知鍵仍按原解碼規則聚到尾串，其游標跟隨輸出位置而可能不單調；首位固定 0，末位固定完整文本長度。`take_raw()` 的提交和清理順序不變。

`Engine::discard_input` / `EngineSession::discard_input` 用於隱私能力變化時無痕清理輸入，包括透傳緩衝、學習鏈和暫存詞彙曝光；`set_private` 只切換寫入開關，保留已輸入的組句。

## crates/qingjian-translate

`Glossary`，本地 TSV 釋義表（詞性 + 譯文）；`LevelTable`，詞彙等級表（`assets/levels/levels-{en,ja}.tsv`，CEFR A1–C2 / JLPT N5–N1，
`uv run tools/corpus/levels.py` 從 `data/levels/` 的原始 CSV 生成，來源與許可見 `assets/levels/README.md`），「統計」頁按級數詞彙用，不進候選。

## crates/qingjian-learning

- `FrequencyLearner`：使用者選擇次數（`user.tsv`）、按輸入串記的選擇（`user-choices.tsv`，詞級排序裡同輸入串選過的優先）、使用者詞（`user-words.tsv`，主詞庫同格式，
  Engine 與主詞庫一起查）、個人英文詞（`user-english.tsv`，回車原樣上屏的英文詞與選過的英文候選，與隨包英文詞表一起出候選且在前；
  原樣上屏的串要像一個詞才學：`MIN_ENGLISH_WORD_LETTERS` = 2 到 `MAX_ENGLISH_WORD_LETTERS` = 15 個字母、切不成完整拼音、
  也不能讀成「拼音頭 + 英文尾」——打不出來時整串上屏的失敗拼音學進去會反過來堵住混輸那條路，越打不出來學得越多）、
  個人敲錯表（`user-typos.tsv`，接受過的 (敲的, 要的) 音節對，詞圖敲錯邊與整段糾錯的代價按它打折）與個人 n-gram（`user-ngram.tsv`，Core `sentence::UserNgram`，
  二元 + 三元線上計數，整句轉換與詞級排序裡與靜態模型插值；Tab 接受的雲端整句按 `sentence::segment_text` 切詞後也記；
  連著選出的兩個詞記夠次數自動造詞進使用者詞，一段拼音分幾次選完的合成詞記兩次也造）。
- `InputLog`：輸入日誌（`input-log.jsonl`，每次上屏一行：敲的鍵、切分、看到的前幾個候選、選了第幾個、來源、糾錯、撤銷，
  Core `InputLogger` trait 的落盤實現，`[general] input_log` 預設開，只寫本機，給離線迴歸評測與個人模型用）。
- `UsageStats`：輸入統計（`usage.tsv`，按天記漢字 / 中文詞 / 英文詞 / 上屏次數，Core `UsageMeter` trait 的實現，Engine 每次上屏 `Usage::of_text` + 按來源定詞數，
  整句按 `segment_text` 切詞數；與輸入日誌無關，偏好設定「統計」頁顯示，`book_scale` 折成幾本《某書》）。
- `VocabularyBook`：詞彙記錄（`user-vocab.tsv`，Core `VocabularyTracker` trait 的實現：學習語言的每條譯詞看到過幾輪 / 上屏過 / ⌥+數字 打出過幾次；Core 私密輸入統一跳過曝光和提交寫入，但仍可讀取已有記錄用於排序和生詞標記；
  Engine `annotate` 據此填 `Sense::fresh`，看到輪次不到 `FRESH_UNTIL` = 3 的譯詞殼裡畫橙色；「看到」按上屏那一刻螢幕上那一頁算，殼每次畫完 `Engine::note_displayed` 告知當前頁）。
- 各表落盤走 Core `storage::write_atomic`（臨時檔案 + fsync + 改名），載入按行容錯（壞行警告跳過，真讀不了殼退回記憶體學習），
  殼啟用期間每 60 秒 `Engine::flush_learning`；IMK 回撥邊界 `imk::catch_panic` 攔 panic、緩衝區字母原樣上屏（見 architecture.md「崩潰不丟」）。

## crates/qingjian-predict

- `CloudPredictor`：`Predictor` trait 的網路實現（async-openai，OpenAI 相容介面，預設 DeepSeek），後臺執行緒防抖 / 快取 / 超時，`submit` / `poll` 非阻塞。
  `PredictConfig` 是配置的 `[predict]` 分節。只在組句中聯想，一次請求給雲端詞（容錯校驗後補進候選第一頁末尾 `[predict] slots` 格，預設 2，不預留不佔位，
  前面的本地候選不挪；排布在 Core `CandidateLayout`）和整句補全（preedit 右側，Tab）；上屏後不聯想，本地歷史不進請求。
  簡拼（半數以上音節是縮寫）的請求 `max_items = 0`，只求整句補全（`prediction::mostly_abbreviated`）：按聲母湊出來的詞大多是生造詞，
  拼音校驗又按首字母序列匹配放行縮寫，攔不住；問字模式的答案不受這條限制。
- `CloudGlossFiller`：釋義兜底（Core `GlossFiller` trait，與 Predictor 分開的執行緒與通道，攢 1.5 秒 / 8 個詞發一次，問過不再問）：
  隨包釋義表沒有的詞庫詞 / 雲端詞上屏後入隊，結果殼每秒 `Engine::poll_glosses` 經 `Translator::learn` 寫進 `qingjian-translate::PersonalGlossary`
  （`user-glossary-<語言>.tsv`，`LayeredTranslator` 個人表優先）；隨雲聯想開關一起開。
- 問字鍵（預設 `u`）開頭是問字模式（`PredictionKind::Question`，答案帶讀音、不校驗拼音），`?` 開頭要 `ModeKeys::question_mark` 開著才算（配置 `[shortcut] question_mark`，預設關，殼用 `Engine::takes_question_mark` 決定空緩衝區的 `?` 是入口還是標點）；`PredictionKind::Translate` 是殼裡快捷鍵觸發的「翻譯選中文字」
  （雙向：漢字為主譯成學習語言，外文譯回中文，`prediction::translation_target`），譯文走結果的 `sentence`。

## crates/qingjian-format

`.qj` 資料容器（`Container` mmap 讀、`Writer` 寫、`Table<T>` / `Text` 零複製檢視、`hash` 可落盤雜湊索引、`Metadata` 名稱 / 許可證 / 署名）。
詞庫與語言模型都能 `write_qj` / 從 `.qj` 開啟，啟動 50 ms；`cargo run --release -p qingjian-dict-convert -- pack dict|lm --name … --license …`
生成 `data/generated/{dict,lm}.qj`，`bundle.sh` 在 TSV 更新時自動重打併只把 `.qj` 打進包。設計見 `docs/design/architecture.md`「資料檔案：`.qj` 容器」。

## crates/qingjian-neural

candle 載入 Transformer（GPT-2 風格 decoder，匯出成
`model.safetensors` + `config.json` + `vocab.json` 三件套）。features `accelerate` / `metal` 換後端，殼用 `metal`。
Metal 缺少 `MTLResidencySetDescriptor`（如 macOS 14）時改用 CPU/F32，避免 candle 建立裝置時 panic；支援該類時仍用 Metal/F16。

Core 的 `sentence::SentenceScorer` 有兩個實現，同一個 trait 拿到**兩種條件**（`context` 游標前文、`keys` 這批路徑共同解釋的那段按鍵），各挑自己訓練時的那個、忽略另一個：

- `CharScorer`（含章·知微，字級模型）用 `context`：給「前文 + 整句」按字累加 log 機率，前文的每層 K / V 快取（`PrefixCache`），
  同一段前文只算一次，每個候選只算自己那幾個字（64 字前文 × 8 條 28 ms，Metal）。
- `P2cScorer`（含章·通變，P2C 模型）用 `keys`，不看前文。**產品端整句重排用它**：凍結集 8322 句上 42.49% 對含章·知微字級模型的 41.80%
  （配對 McNemar p=0.003，留出驗證 λ 未過擬合），中位延遲 18.5 對 22.0 ms，而且同一個模型還能造詞。
  字級模型沒有下崗，它是移動端「游標聯想」那類 `P(下一段 | 前文)` 功能的基礎——P2C 的 `<eos>` 明確切斷上文，結構上做不了那件事。

Engine 側在 `engine/rescoring/`：接了打分器就取 Viterbi 前 `RESCORE_PATHS` = 6 條路徑按 `路徑分 + λ·(神經分 − 靜態二元分)` 重排（λ `NEURAL_WEIGHT` 0.5，
個人 n-gram / 使用者加分 / 代價不動）。**神經分只決定名次，不寫回 `Conversion::score`**：P2C 打的是 `log P(漢字 | 拼音)`，
比靜態分高十幾二十 nat 且抬升隨句長變化，寫回去會汙染跨讀法的比較（拼寫糾錯的原樣 vs 糾正、混輸的頭段 vs 整段），
那兩處的門檻都按靜態尺度定；按貝葉斯展開也只在同一串按鍵下可比。分走「(前文, 按鍵) + 文本 → 神經分」快取 `NeuralCache`（兩個條件任一變了整張作廢）；
按鍵取的是**當前這批 patterns 覆蓋的字母**而不是整個作用域——英文尾巴那條只轉換 head，拿整段當條件會讓它憑空背上沒覆蓋的字母；
同步打分器（`with_sentence_scorer`，CLI 評測）當場補分，
非同步的（`with_async_sentence_scorer`，後臺執行緒 `RescoreWorker`）查詢不等模型：缺分的記下來，殼停鍵後 `request_rescoring`、`poll_rescoring` 到了再 `query` 一次。
前文優先用殼給的應用游標前文（`set_rescoring_context`），沒有用本會話最近 64 個上屏字元。CLI `--neural <匯出目錄>`（`--neural-weight` / `--neural-context` / `--neural-async`）。

P2C 教師強制打分 `CharScorer::score_p2c(keys, texts)`：字首 `<eos> + 完整拼音 + <sep>`，只累加候選字元的 log 機率（不計 EOS），
共享拼音字首 KV，候選批次前向；超過上下文時報錯，不截斷拼音。CLI 實驗入口 `--eval-p2c data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm --eval-text data/eval/sentences.tsv`，
用 `--neural-weight` 調 λ。評測介面卡與產品端 `P2cScorer` 同一條件，只是推理出錯時直接終止（不靜默回退到基線冒充成功）。
`--eval-details <輸出.jsonl>` 為任意整句評測儲存逐句首選、整句候選、全部候選與查詢耗時，支援配對比較，仍不上屏、不學習。

P2C 自由生成實驗：`--eval-text <集> --eval-generate data/models/hanzhang-tongbian/hanzhang-tongbian-small.qjm`（`P2c::convert` 束寬 5，不經詞圖不經重排），
量的是重排這層架構的上限代價；2081 句上首選 51.9% / 前五 73.5%，對比重排路徑的 42.3% / 45.5%，兩條路互補（只有生成對 16.5%、只有重排對 6.9%），
代價是 `O(生成字數)` 次序列前向。數字與取捨見 [neural-rescoring.md](neural-rescoring.md)。

**模型直接生成的整句**（`engine/query/generating.rs`，候選型別 `CandidateKind::Generated`）：詞圖只會把整段按鍵讀成拼音，
中英混輸（`yongdockerbushuhenfangbian`）與生詞在它那裡根本沒有路徑，出來的只能是把英文段硬讀成拼音的廢話（用的哦乘客仍不熟很方便）。
詞圖讀不通整段時改問 `SentenceScorer::generate`（P2C 走 `P2c::convert`，字級模型返回空），生成的整句插在詞圖那幾條前面。
「讀不通」三條：最優切分裡有不完整音節、切分沒覆蓋到末尾（`woyongvscodexiedaima` 的 `v` 起不了音節）、拼寫糾錯生效
（`womaileyigeiphone` 的 `phone` 被當成敲錯的 `paone`）。拼音乾淨的輸入一條都不觸發，常態零成本；
非同步打分器下走同一次 `request_rescoring`（`Job.generate`），按鍵回撥不等它。
`GENERATE_BEAM` = 5、`GENERATE_MAX_CHARS` = 32、`GENERATED_CANDIDATES` = 2、`MIN_GENERATED_LETTERS` = 6；雙拼 / 注音不走（按鍵不是模型見過的字母）。
生成的候選沒有音節對齊，上屏吃掉整段作用域、不記學習。漏的是「英文詞本身就是合法拼音」那種（`zhegeapihenhaoyong` 的 `api` 讀成 `a pi`），
光看切分分不出來。

整句候選不止一條：`SENTENCE_CANDIDATES` = 3 條，重排後的前幾條路徑都進候選表，四個音節
（`ALTERNATE_MIN_SYLLABLES`）以下不給備選（那幾格留給詞級候選）。凍結集上前三 42.4% → 45.5%，無模型時 38.1% → 43.2%。
`Engine::sentence_paths` 的 `want` 引數控制算幾條：拼寫糾錯對每個糾正候選都要轉一次，那條路仍然只算一條。

冷啟動字詞實驗：`--eval-cold <樣本.jsonl> --cold-output <新結果.jsonl>` 強制用預設配置與記憶體學習器，
與 `--config` / `--user-dict` / `--predict` / `--replay` 等衝突，不載入或刪除個人資料。
每行樣本欄位 `id/text/keys/source/category`；按完整拼音查候選，不給上文、不上屏，解析失敗也記錄在分母中。
領域詞庫用 `--extra-dict` 顯式指定，`--neural` 可測字級重排；`--cold-model` 可同時測 P2C beam 5、固定上限 16 字的生成。
實驗合併策略保留原首選，將生成的新增漢字候選插在其後，再接原候選；儲存全部候選、詞庫文本/讀音覆蓋及耗時。
該策略只量候選覆蓋和排序代價，沒有改產品行為。生成搜尋不讀取目標字數，候選輸出拒絕覆蓋已有檔案。

## crates/qingjian-lm

`BigramModel`，Core `sentence::LanguageModel` trait 的實現，從 `data/generated/lm.qj`（或 `lm-unigram.tsv` / `lm-bigram.tsv`）載入
（沒有這兩個檔案就退化為一元詞頻整句）。資料由 `tools/corpus/parquet_to_text.py`（uv 指令碼，HF parquet → 簡體純文本）加
`cargo run --release -p qingjian-dict-convert -- bigram --phrases assets/lexicon/phrases.tsv --phrases assets/lexicon/domain_words.tsv --brand assets/lexicon/brand.tsv --brand assets/lexicon/mixed_words.tsv data/corpus/*.txt` 生成；語料在 `data/corpus/`（gitignore）。
短語層不當 token 統計（分詞時摘掉、統計完按成分合成一元 / 二元，短語得分等於原來兩個詞的路徑，見 `bigram.rs` 模組註釋），品牌詞按給定次數寫進一元與句首二元。

## crates/qingjian-platform

輸入方案是**兩條獨立的軸**：`Scheme`（拼音側：全拼 / 雙拼七套 / 大千注音 / 關，`[general] scheme`）
與 `[general] wubi`（形碼側：空為關 / `wubi86`）。兩邊都開就是**混輸**（`GeneralConfig::mixed`）。
`scheme_label(pinyin, wubi)` 是狀態條顯示的方案名（形碼在前），做成自由函式而不是存進 `RouterConfig`——
存了會與那兩項冗餘、手搓配置的地方就漂移（狀態條那條測試正是這麼發現的）。

演進：2026-09-16 之前是 `shuangpin` + `zhuyin` 兩個鍵表達同一個維度，先併成單選的 `scheme`，
同一天又拆成兩條軸（單選的 `scheme` 表達不了混輸）。舊鍵（`shuangpin` / `zhuyin` / `scheme = "wubi86"`）
都還在讀、檔案不自動改寫。


`Config`（TOML 配置檔案，`[general]` / `[shortcut]` / `[fuzzy]` / `[dictionaries]` / `[apps]` / `[predict]` 分節，首次執行寫模板，
`set_value` 用 toml_edit 原地改鍵保留註釋；`[model] enabled` 本地整句模型開關，`LocalModelConfig`；
中英模式兩項：`[shortcut] switch_mode`（`SwitchKeys`：勾選 shift / control / ctrl+alt+space，可多選，老配置的單個字串照讀）與 `[general] english_mode`（內建英文模式總開關））；
`extra_dictionaries` 列出 / 載入隨包領域詞庫與使用者 `dicts/`
（mac 殼與 Windows Server 共用，同名 `.qj` 優先於 `.tsv`）；`code_tables` 同構地列出 / 載入隨包根 `codes/` 與使用者 `codes/` 的碼錶
（`[aux_code] disabled` 是黑名單，`[general] aux_code_key` 預設 `;` 且校驗後退回預設、`aux_code_show` 是顯示碼開關）；
`protocol` 模組是 Windows Server ↔ TSF DLL 的 IPC 協議型別
（`ClientMessage` / `ServerMessage` / `Frame` / `PreeditSegment`，全 serde，兩端共用，見 `docs/design/architecture.md`「Windows：TSF」；
協議版本見 `PROTOCOL_VERSION`，`PreeditKind::AuxCode` 對應 Core 的 `MarkedKind::AuxCode`，`Frame.aux_code_show` 隨幀下發顯示碼開關）。

## crates/qingjian-render

橫排矩陣：視口與高亮移動在 Core `candidate::layout::Grid`（`GRID_ROWS` = 6，列寬 `Grid::column_ems` 按整份候選估、單格封頂 `MAX_CELL_EMS` = 4 字寬）。
**青簡渲染器還不會畫矩陣**：main 上的 `renderer/matrix.rs` 建在手算座標的舊渲染器上，合進主題分支時隨舊渲染器刪了，場景樹這邊要另做一個矩陣視窗模板
（固定列寬、超寬截尾、網格下一行資訊）；移植之前 mac 殼只在系統繪製下展開矩陣（`Host::grid_keys` 多一個 `!uses_bitmap()` 條件），`Frame::columns` / `column_ems` 渲染器暫時不讀。

自繪渲染器：候選窗一幀 + 主題 → 預乘 RGBA 點陣圖。主題是 `themes/<id>/theme.json`（內建青簡綠 / 系統藍 / 微信綠 / 櫻花編進 crate；新增內建主題前先查主題倉庫 qingjian-themes 有沒有同名 id，加了之後同步它 `tools/check.py` 的 `RESERVED_IDS`，否則那個使用者主題會被拒絕載入；圖片也可以 `include_bytes!` 編進去（`BUILTINS` 的檔案表），`Theme::builtin(id)` / `builtins()` 取；沒寫 `extends` 的主題都以青簡綠為底（`theme/mod.rs` 的 `ROOT_THEME`），使用者主題還能取到所繼承內建主題的圖片；使用者主題在 `<資料目錄>/themes/<id>/theme.json`，`ThemeLibrary` 合併並按目錄戳熱載入；`theme/file/` 是 serde 模型，`theme/jsonc.rs` 解析前去掉註釋與尾逗號，格式見 `docs/design/theme.md`「格式（schema 1）」），`renderer/build/` 按模板例項化場景樹（`scene/`：Taffy 佈局樹 + 每節點一個 `Visual`，豎排三列用 grid、其餘 flex，高亮條與游標絕對定位；佈局不取整，文字測量按節點快取；表格一行裡的格子按第一段文字的基線對齊，`scene/baseline.rs` 補上邊距再排一遍），再按樹序畫（節點的投影 / 內陰影在 `scene/draw/effect.rs`：把節點自己的畫面畫進離屏圖取 alpha，三遍盒式模糊；視窗陰影就是根節點的投影；點陣圖按 `scene/extent.rs` 並出的畫出範圍開（投影、描邊、負 inset 伸出視窗的裝飾都在裡面），殼按 `Rendered::content_*` 對齊，mac 面板關係統陰影；`Rendered::hits` 是候選與各條譯詞的點選區域（`renderer/hit/`，譯詞按義項分隔數分條，橫排底下那行算高亮候選的），殼只在這些區域收滑鼠、其餘穿透）；過渡在 `animation/`（緩動、插值）與 `renderer/{animate,retained}.rs`：`Renderer` 留住上一幀，按節點 `id` 配對算過渡，`Rendered::next_frame` 告訴殼多久後調 `tick`（mac `candidates/animation.rs` 的 NSTimer、Windows UI 執行緒訊息迴圈截 `WM_TIMER`），視窗收起調 `forget`；動畫幀只重畫、不重建樹，效果遮罩與圖片框按鍵快取（`scene/cache_key.rs`）；迴圈動畫 `animation/keyframes.rs` 求姿態，帶姿態節點整棵子樹畫進快取小圖再變換貼上，迴圈幀走區域性重畫（`renderer/partial.rs` + `scene/split.rs` 分段模式）；tiny-skia 柵格 + cosmic-text 文字（fontdb 按平臺清單隻載入幾個字型檔案、不掃系統；
指令碼字型檔案要先過輪廓檢查，任一面帶 `glyf` / `CFF ` 才載入——macOS 26+ 的蘋方換成私有 `hvgl` 可變輪廓，swash 讀不出字形，
這種檔案跳過，簡體回退到 Songti.ttc），
自己解析 `trak` 字距表、按主題 gamma 加深筆畫；主題樣式的字族在 `theme/font/families.rs` 去重成字族表（樣式裡存序號），`TextPainter::use_families` 換主題時給每條鏈挑裝了的字族，主題要的系統字族由殼查檔案經 `Renderer::load_theme_fonts` 載入（mac 同 `bitmap/font_files.rs`、Windows 同 `system_fonts::family_files`），隨主題帶的字型在 `theme/assets.rs` 核路徑；主題圖片 `.svg` 結尾的走 `theme/svg_image.rs`（resvg 0.48，與渲染器同一個 tiny-skia，關掉 text / raster-images 與外部檔案讀取），畫時按畫素尺寸柵格並快取；cosmic-text 打了 `opsz` 光學字號補丁（qingjian-team/cosmic-text 分支 `qingjian-opsz`，workspace `[patch.crates-io]` 釘 rev）。
`examples/preview.rs` 出 PNG 與真機截圖並排比、`--measure` 與 AppKit 對寬度、`--theme <目錄>` 畫使用者主題（系統字族在 `--font-dir` 裡按名字找）。
快照測試 `tests/snapshots.rs`：`tests/scenes/` 的樣例幀（與 preview 共用）逐畫素比 `tests/snapshots/<os>/` 的基準，字型環境（`fingerprint.txt`）對不上或沒有基準時跳過；
改了渲染就 `QINGJIAN_UPDATE_SNAPSHOTS=1 cargo test -p qingjian-render --test snapshots` 重出基準，變了的 PNG 看過再提交，失敗時差異圖在 `target/tmp/render-snapshots/`。mac 殼 `candidates/bitmap/` 貼點陣圖，`[general] renderer = "system"` 切回 AppKit 繪製
（過渡期退路，偏好設定「候選視窗」頁可選）；`[general] font` 是候選窗字族名（空為系統字型，`bitmap/font_files.rs` 用 CoreText 按字族名找檔案只加載那幾個，沒裝就回系統字型；
設定頁 `preferences/font_picker/` 是搜尋框 + 列表）。設計與驗收見 `docs/design/rendering.md`。

主題預覽圖（`preview/`）：`preview::write_previews` 用 `sample.rs` 的 nihao 兩幀把主題目錄畫成 2 倍的 `vertical.png` / `horizontal.png`（沒鎖定外觀再加 `vertical-dark.png`），單張失敗只記在它的 `Shot` 裡；`preview::run` 是兩個殼 `--theme-preview <主題目錄>` 的入口（mac `main.rs`、Windows Server `main.rs`，Windows release 是 GUI 子系統，直接從 cmd 跑時先 `AttachConsole` 掛父程序控制台），寫進 `<主題目錄>/preview/`。主題倉庫的 `tools/preview.py` 找本機裝的青簡調它，舊版不認引數會照常起服務，指令碼 30 秒超時按版本太舊提示。

## crates/qingjian-update

檢查更新（設計見 `docs/design/update.md`）：`index/` 是索引的型別、下載（`fetch.rs`，複用 workspace 的 reqwest + 單執行緒 tokio，20 秒超時、2 MB 上限）與驗籤
（`signature.rs`，`PUBLIC_KEYS` 列表，`verify_strict`）；`checker/` 是排程（`Checker::poll` 由殼的每秒定時器調，到點起一次性執行緒）、落盤狀態 `UpdateState`（`update.json`，先寫臨時檔案再改名）
與查到的結果 `Available`。`Version` 自己實現語義化版本比較，不引 semver。`[update]` 配置與 `UpdateChannel` 在 `qingjian-platform`。
`examples/check.rs` 手動走一遍；`tools/release-sign` 是發版側的 keygen / sign / verify。

## apps/cli

測試工具，`cargo run -p qingjian-cli -- kaifa`。

- `--predict` 強制開雲聯想並等結果列印，互動模式下上屏後也聯想。
- `--wubi <碼錶>` 用形碼碼錶（`詞\t編碼\t詞頻` 的 TSV）替代拼音：按鍵當編碼按字首查表，候選不帶音節，上屏吃掉整段編碼。
- `--typing` 逐鍵計時（效能測試用 release 構建跑，目標每鍵 10 ms 以內）。
- `--chinese-first` 開啟中文優先（`[general] chinese_first = true` 的排法），配合 `--replay` 比兩種英文詞位置。
- `--replay <input-log.jsonl>` 回放評測：把日誌裡每次上屏的鍵重新餵給引擎，按來源算首選 / 前五命中率、平均名次、不在候選的條數，列印沒命中的例子（`--misses N`）；
  只在記憶體裡學習不寫檔案，加 `--user-dict` 可帶上現有學習資料。
  **按日誌記的方案逐條切換**（`replay/scheme.rs` 的 `SchemeSwitcher`，讀每條的 `scheme` 欄位）：整份日誌通常只有一套方案，
  所以只在方案串變化時才換碼錶（`Engine::set_code_table` 收所有權，換一次要克隆 8.9 萬條）。日誌裡是形碼但沒給 `--wubi` 時，
  那部分只計數——拿拼音的讀法喂形碼的鍵會算出看著像真的、實則無意義的命中率。
  含觸發鍵的日誌行按殼那樣逐鍵重喂（進輔碼態、碼段進碼段），
  並多打一行「輔碼選詞 N 條，其中同拼音純輸入首選命中 M 條」（學習閉環的尺子）。
- `--aux-table <路徑>`（可多次）裝輔碼碼錶（`.qj` 或 `詞<Tab>碼` TSV）；互動模式與查詢模式都按殼的方式逐鍵喂入，
  配的觸發鍵進輔碼態、之後的字母進碼段，候選行會帶上命中的碼。
- `--tune 名=值`（逗號分隔）覆蓋個人 n-gram 插值與敲錯代價的常數掃網格（名字見 `apps/cli/src/tuning.rs`，Core 側是 `Engine::set_interpolation` / `set_typo_costs`，殼只用預設值）。
- `--eval-text <文本>...` 裝了碼錶（`--aux-table`）時多打一行碼錶覆蓋率：詞頻前 10,000 與全庫兩段命中比例（與匯入統計同源）。
- `--eval-text <文本>...` 整句評測：把使用者自己寫的中文文本按標點切句、按詞庫讀音轉成全拼，冷啟動餵給引擎看整句能不能還原原句
  （首選命中率 / 字準確率 / 查詢耗時；不依賴日誌裡當時選了什麼，給整句排序與語言模型的改動當尺子），`--eval-save` 凍結成 `句子\t拼音\t上文` 三列檔案，
  之後直接 `--eval-text` 它保證比的是同一份句子（本機的在 `data/eval/sentences.tsv`）。排序、整句、糾錯的改動先跑它們再合。

## apps/macos

IMK 輸入法，原始碼按 `app / host / imk / candidates / menubar / preferences` 分目錄。

- 輸入法選單（狀態項 + 系統輸入源選單）與偏好設定視窗都是配置檔案的前端：只寫 `config.toml`，`Host::apply_config` 一條通路熱載入，啟用期間每秒看一次檔案 mtime。
  輸入方案（`[general] scheme`）也在這裡裝配：雙拼 / 注音設給引擎，形碼額外按 `paths::code_table_path()` 掛碼錶
  （使用者目錄 `wubi/wubi86.tsv` 優先，包裡 `Resources/wubi/` 兜底；找不到只警告並按拼音跑）。
- `apps/macos/scripts/bundle.sh --install` 打包安裝到 `~/Library/Input Methods/`（開發用），`--pkg` 做分發用的 pkg（裝 `/Library/Input Methods/`，postinstall 跑 `qingjian-macos --register`
  註冊、啟用並切成當前輸入源；簽名 / 公證靠 `QINGJIAN_SIGN_IDENTITY` / `QINGJIAN_INSTALLER_IDENTITY` / `QINGJIAN_NOTARY_PROFILE`，沒設就 ad-hoc；`QINGJIAN_TARGET` 指定架構，
  成品 `target/pkg/qingjian-<版本>-macos-<arm64|x86_64>.pkg`）；`scripts/uninstall.sh` 解除安裝。
- 日誌在 `~/Library/Logs/Qingjian/`（按天分檔案留 7 天，刪了會重建），使用者資料與配置在 `~/Library/Application Support/Qingjian/`。
- 配置項：雲聯想 `[predict]`（偏好設定「雲服務」頁有「測試連線」按鈕：`qingjian_predict::ConnectionTest` 起執行緒發一條最小請求，`Host` 用獨立定時器 `CloudTestMonitor` 輪詢結果顯示到視窗底部；
  `reasoning_effort` 預設 `none`，DeepSeek V4 預設思考，不關正文為空）；模糊音 `[fuzzy]` 預設都關；`[general]` 學習語言（`off` 不顯示譯文）/ 每頁候選數 / 翻頁鍵 / 外觀 / 豎排橫排 / 拼音顯示位置 /
  英文模式候選開關 / 中文優先 `chinese_first` / 中文模式英文詞 `english_in_chinese`（`Engine::set_english_in_chinese`）/ emoji 候選 `emoji`（`Engine::set_emoji_candidates`）/ 雙拼方案 `shuangpin`（小鶴 / 自然碼 / 微軟 / 搜狗 / 智慧ABC / 小浪 / 首道，空為全拼）/ 日誌級別 `log_level`（預設 info 不含敲的內容，debug 逐鍵記，熱切換）/ 輸入日誌 `input_log`；
  `[shortcut]` 模式鍵 v / u、`question_mark`（預設關，開了空緩衝區敲 `?` 進問字）、上屏第一 / 第二個譯詞的修飾鍵 `translation` / `translation_second`、刪候選 `delete_candidate`（預設 shift，使用者詞整刪、詞庫詞清學習）、翻譯選中文字 `translate_selection`；
  `[apps] english_candidates_on` 按應用標識列出英文模式給候選的應用（白名單，預設瀏覽器 / 聊天 / 辦公 / 筆記，`*` 字首匹配，`["*"]` 處處都給），`english_candidates_off` 從中排除；
  `[dictionaries] domains` 開啟隨包的領域詞庫（`Resources/dicts/` 11 本，預設只開 `idioms`），`disabled` 關掉使用者目錄 `dicts/` 裡的某本匯入詞庫；
  偏好設定「詞庫」頁隨包的可開關、匯入的可開關 / 移除，可匯入 TSV / Rime yaml / .qj。
- 系統文本替換（系統設定「鍵盤 → 文本替換」）：`host/config/text_replacements.rs` 從 `NSUserDefaults` 全域性域讀 `NSUserDictionaryReplacementItems`
  （每條 `{ on, replace, with }`），啟用輸入法時重讀，變了就經 Core `merge_replacements` 並進配置裡的自定義短語再 `set_custom_phrases`；
  `[general] system_text_replacements` 開關（預設開，「自定義短語」頁勾選框），內容可能含證件號、地址，日誌只記條數。
- 輸入法程序由 launchd 拉起，看不到 shell 的環境變數：金鑰寫進配置同目錄的 `.env`（`QINGJIAN_API_KEY=...`，輸入法啟動時 dotenvy 讀入）或 `config.toml` 的 `api_key`。
- 本地整句模型：`bundle.sh` 把 `data/models/hanzhang-tongbian/`（或 `QINGJIAN_P2C_MODEL_DIR`）打進 `Resources/models/hanzhang-tongbian/`，通變優先；知微放 `Resources/models/hanzhang-zhiwei/` 作回退，使用者目錄的對應模型優先於隨包同類模型，舊使用者目錄仍可讀取。`host/model/mod.rs` 在後臺執行緒載入並預熱（首次 Metal 編譯）後
  `set_async_sentence_scorer` 接上，`refresh` 每鍵先讀應用游標前 64 字給 Engine 當前文、查詢後 `schedule_rescoring`，`RescoreMonitor` 停鍵 80 ms 請求、20 ms 輪詢，
  結果到了重查一次只重畫當前頁（翻過頁 / 動過高亮不動）；「雲服務」頁有開關（`[model] enabled`）。
- 端到端驗證可用 `osascript` 的 System Events 往 TextEdit 發按鍵再讀迴文本（終端需要輔助功能許可權；輸入法得在中文模式）。

## apps/windows

一個產品兩個 package：`server`（Server 程序：IPC 分派 + Engine + 命名管道 + 自繪候選窗與懸浮狀態條）與 `tsf`（TSF 文本服務 DLL，lib 名固定 `qingjian_tsf`），
外加 `settings`（WinUI 3 設定程式，含「輔碼」頁）與 `installer`（Inno Setup）。
Server 側輔碼接線：`RouterConfig.aux_code_key` / `aux_code_show`（`apply_config` 熱載入）、`assembly` 從隨包 `codes/` 與使用者 `codes/` 裝碼錶
（`[aux_code] disabled` 過濾，`code_tables::load`）、按鍵在 `dispatch/key/input.rs` 分流（觸發鍵進輔碼態、輔碼態裡字母進碼段、
大寫（`shift_letter = "compose"`）先清碼段回拼音態再進緩衝區、標點先上屏高亮候選再轉全形）、候選窗 `ui/candidates/row.rs` 的 `Row.code` 把碼用方括號括起來緊跟在候選詞後面（不進 annotation），拼音行 `view.rs` 給碼段加下劃線。
熱載入的 `dicts/` 與 `codes/` 目錄與啟動同款（修過一處傳基礎目錄的錯）。不合成一個 crate，因為 DLL 不能帶 Engine 的依賴樹，見 `apps/windows/README.md`；
協議型別在 `qingjian-platform::protocol`，設計見 `docs/design/architecture.md`「Windows：TSF」。
輸入方案由 `[general] scheme` 一處決定，Server 啟動與熱載入各裝配一次；形碼的碼錶用 `dispatch::code::find_code_table` 找
（使用者目錄 `wubi/wubi86.tsv` 優先，隨包 `assets/wubi/wubi86.tsv` 兜底——走 `assets/` 與 emoji / levels 一致，開發佈局也對得上），**路徑在啟動時定下、熱載入不重新找**。
選了形碼卻沒有碼錶檔案時只警告並按拼音跑——配置說五筆、引擎還在拼音是靜默錯位，寧可吵。
中英模式的兩項設定（`[shortcut] switch_mode` 切換鍵：勾選 shift / control / ctrl+alt+space，`[general] english_mode` 內建英文模式開關）
由 Server 經協議下發給 DLL（`InputSettings`，見下文「按鍵行為設定」），改完在下一拍（約 320 ms）生效；
`ctrl+alt+space` 走 TSF 保留鍵登記（`com/key/preserved.rs` 的 `GUID_SWITCH_MODE`）。系統的 Ctrl + Space（「輸入法/非輸入法切換」）不進勾選項但適配它：
它翻的「輸入法開 / 關」compartment（`com/mode/sink.rs`）關 = 英文、開 = 中文，我們切模式時把開關寫成一致；開關一變也作廢被截走 Space 的那次「單擊 Ctrl」。
語言 profile 登記兩個（`tsf/src/com/profile/language.rs` 的 `PROFILES`）：「中文(簡體)」0x0804 的「青簡」與「中文(台灣)」0x0404 的「青簡」，同一個 CLSID、各自固定的 profile GUID，
`DllRegisterServer` 一起登記、共用一張圖示；繁體那個不看 `[general] traditional` 就出繁體。DLL 連上 Server 後用 `ITfInputProcessorProfiles::GetActiveLanguageProfile` 查當前是哪個，
是繁體才發 `ClientMessage::Profile`（協議 v9 起；開會話預設按簡體），同一執行緒在兩個 profile 間切換由 `ITfActiveLanguageProfileNotifySink` 收到再報；
Server 按會話記（`SessionInfo.traditional`），聚焦時 `traditional_output()` = 配置開關 ∨ 會話 profile 設給 Engine，熱載入也走它，候選窗主題的 `mode.traditional` 同源。
模式全域性一份、存在 Server（`Router.english`），DLL 啟用 / 得到焦點 / 輪詢時 `SyncMode` 取回，使用者切了 `ModeChanged` 報上去。會話號用執行緒 id（`com::session_id`）——TSF 的 client id
各程序都是同樣那幾個值，拿它當會話號會在 Server 那邊撞號。四條切換入口都匯到
`service/mode.rs::set_english_mode` 一處攔住；狀態條點選在 Server 側（`dispatch/status/mod.rs`）按同一項攔，
設定介面在 `settings/src/panel/pages/general.rs`。

TSF 原有數字 / OEM 標點 / 空格鍵碼按當前佈局用 `ToUnicodeEx` 解析（bit 2 避免改變鍵盤狀態），
僅接受單個非代理項 UTF-16 單元。字母、小鍵盤和 AltGr 處理不變，不保證組合音符輸入。
拼音顯示位置（`[general] preedit`）在 Windows 上分兩處落地：Server 把它讀進 `RouterConfig.preedit` 並隨 `Frame.preedit_mode`
下發給 DLL，DLL（`com/service/key_sink.rs`）按 `inline()` 決定要不要放行內拼音，Server（`ui/candidates/render_data.rs::window_preedit`）
按 `in_window()` 決定候選視窗頂部畫不畫拼音行；`window` 模式沒有組句範圍，游標矩形改從 `com/edit/anchor.rs::caret_rect`（當前選區）量。
連不上 Server 時 DLL 自己拉起它（`tsf/src/com/service/launch.rs`）：`ShellExecuteW` 起與 DLL 同目錄的 `qingjian-server.exe`
（`uiAccess=true` 的 exe 用 `CreateProcess` 報 740），程序內 5 秒冷卻 + 跨程序命名互斥體防止砸出一串 Server；
起完清掉重連退避，下一鍵就試。Server 只在登入時由「啟動」資料夾拉起，中途掛了以前只能等下次登入。

詞庫匯入（設定「詞庫」頁）走 `qingjian-dictionary::import` 轉成 `.qj`（空詞庫拒絕），多選批次、成功的從 `[dictionaries] disabled` 摘掉、頁面顯示每個檔案的結果；
Server 每次輪詢比對使用者 `dicts` 的路徑 / mtime / 長度快照，配置沒變也過載新增、同名更新與移除；配置解析失敗時詞庫沿用上次有效的開關（#36）。

中英切換鍵（`[shortcut] switch_mode`）與內建英文模式開關（`[general] english_mode`）得在**按鍵到達之前**就知道
（單擊判定在 `OnTestKeyUp`、是否登記語言欄按鈕），但 DLL 跑在每個應用程序裡、拿不到 Server 那份配置，
`%APPDATA%\Qingjian` 對 AppContainer 裡的商店應用也讀不到（那是給輸入日誌和 `.env` 用的目錄，不該加 ACE）。
所以由 **Server 讀配置、經協議下發**（`InputSettings`：`OpenSession` 回包帶一次，之後每拍 `SyncMode` 跟著走），
DLL 不讀檔案、不查 mtime。`SessionOpened` 只回過協議版本對得上的 DLL——老的 `open` 是隻寫不讀，
多回一條會被它當成下一次 `Poll` 的應答而報錯，那條連線就廢了；老 DLL 從 `ModeSync` 那一拍也能拿到同一份（新欄位直接忽略）。

## assets

- `assets/sample/`：手寫樣例詞庫與釋義表，不是產品資料。
- `assets/wubi/wubi86.tsv`：五筆碼錶（`dict-convert wubi` 生成，來源與許可見同目錄 README，Apache-2.0）。
  `bundle.sh` 拷到 `Resources/wubi/`，Windows 安裝器拷到 `{app}\assets\wubi\`（與兩邊 Server 的找法對齊：macOS `paths::code_table_path`、Windows `dispatch::code::find_code_table`）。
- `assets/emoji/emoji-zh.tsv` / `emoji-en.tsv`：Unicode CLDR 中文 / 英文 annotations 轉出的 emoji 表（Unicode License v3，可釋出；中文詞與英文詞各配 emoji，兩張表載入時合成一張），
  `cargo run --release -p qingjian-dict-convert -- --out-dir assets/emoji emoji --language zh data/cldr/annotations-zh.json data/cldr/annotationsDerived-zh.json`（en 同理）。
- 英文詞表詞頻：`uv run tools/corpus/english_frequency.py data/generated/english.tsv -o data/generated/english-frequency.tsv`，再 `... english <詞表> --frequency <那個檔案>`。

## tools/gloss-gen

用 LLM 批次生成釋義表：`cargo run --release -p qingjian-gloss-gen -- generate`（金鑰讀 `QINGJIAN_API_KEY`，結果 JSONL 在 `data/generated/`，不進 git、可續跑，`--limit 80` 試跑）
再 `... export`（寫 `glossary-{en,ja,vi}.tsv`，產品資料在 `assets/glossary/`，見那裡的 README；格式 `詞\t詞性. 譯詞[|假名]`）。只補越南語時 `generate --languages vi --out data/generated/gloss-vi-llm.jsonl`，再對這個 JSONL `export`。CLI 與 bundle.sh 用的是這些檔案；打包時 `dict-convert pack glossary --language vi` 會寫 `glossary-vi.qj`。

## tools/dict-convert

產品資料的生成工具，輸出到 `data/generated/`（gitignore）。

- `lexicon`：從 `assets/lexicon/`（自建詞庫源：規範字 + 常用詞 + THUOCL 領域詞）加 Unihan 讀音（`data/unihan/Unihan_Readings.txt`）、LLM 多音字標註（`gloss-gen pinyin`，
  結果 `data/generated/pinyin-llm.jsonl`，不進 git）、語料詞頻（`lm-unigram.tsv`）建基礎詞庫 `dict.tsv`（8.7 萬條），並把 THUOCL 領域詞按語料次數 < 50 拆成
  `dicts/<領域>.tsv` + `.qj`（11 本、13 萬條，`--domain-keep-min`），流程見 `assets/lexicon/QINGJIAN.md`；Unihan 只給 `n` / `ng` 的字（嗯）按輸入習慣收成 `en`；`--extra-words` 併入人工挑的領域詞 `assets/lexicon/domain_words.tsv`（2026-09-27 起該檔案另含按 jieba 詞表（MIT）對照出的缺失高頻詞：讀音逐字取 Unihan、多音字逐條人工判定（判定明細與理由在 `assets/lexicon/00_meta/polyphone-judgments.tsv`），詞頻按 log-log 迴歸從 jieba 次數折算；另含「姓氏+總」稱謂層 98 條與符號詞 6 條，定值依據見檔案頭注。這些詞進語言模型同樣走 `bigram --phrases`，改了詞庫發資料時要重跑 bigram，否則新詞只拿兜底分、會被同音的 lm 詞壓過）。
- `english`：轉 `assets/lexicon/05_english/00_all_words.tsv`；同編碼優先保留含大寫的專名寫法（Windows ≠ windows），
  展示寫法補充表 `07_display_forms.tsv` 後置讀入；`cedict`：釋義表備用來源。中英混雜詞源在 `assets/lexicon/mixed_words.tsv`（`lexicon --extra-words`）。
- `wubi`：Rime 形碼碼錶（`.dict.yaml`，極點 86 五筆）→ `詞\t編碼\t詞頻`（`wubi.rs`，`--name` 決定檔名，預設 `wubi86.tsv`）。
  **碼錶自帶的權重不用**——那是碼錶順序不是語料詞頻，用了同一個詞在形碼下和在拼音下會排得不一樣；詞頻從青簡詞庫按**詞面**交叉回填，
  詞庫裡沒有的詞給 `UNKNOWN_FREQUENCY = 1`。解析複用 `qingjian_dictionary::import::rime`（與使用者匯入 Rime 詞庫同一個解析器，
  形碼碼錶的第二列是編碼，格式一樣）。詞庫還沒收的詞不收（碼錶整張進，不做取碼推導，見 `docs/plan/wubi.md`）。
- `bigram`：統計語料；`--phrases` 給短語層、`--brand` 給品牌詞（`assets/lexicon/brand.tsv`，青簡 210）與中英混雜詞（`mixed_words.tsv`，C盤 / B站：合成計數要成分詞在語料裡，C 不是 token，只能直接給一元，次數對著同音競爭詞定），領域詞也走合成計數（語料裡只有幾十次的詞當 token 統計會吸走成分詞的二元證據）。
- `mine`：從語料挖詞庫沒收的高頻詞並過濾（`oov_filter.rs`：虛詞規則 + 相鄰字對 PMI≥3，`--candidates` 只重過濾）。
- `phrases`：挖短語層（兩遍掃語料：相鄰兩詞、兩段二元都夠頻的相鄰三詞，總次數與對話語料次數都 ≥ 2000 + 邊界規則，讀音由成分詞拼出；我的 / 不知道 / 有沒有 這類常用詞表不收的組合，
  `assets/lexicon/phrases.tsv`；詞庫已併入過短語時重跑加 `--refresh`）。
- `pack dict|lm|glossary|codes`：打 `.qj`（釋義表也進容器；`codes` 是唯一帶計算的一種，見下）。
- `stroke`：CNS11643 全字型檔筆順（`data/cns/`，官方 Properties.zip / MapingTables.zip 解出，gitignore）+ 大陸序覆蓋表
  `assets/stroke/prc-rules.tsv` → `data/generated/codes/stroke.tsv`（隨包筆畫表的源資料：7,991 字、127 KB，
  1 橫 2 豎 3 撇 5 折 n 點捺；首筆按《通用規範漢字筆順規範》GF 0023—2020 全對：門字頭 / 戶→戶 兩條字首規則 + 66 行整字覆蓋，阝第二筆隨規範改豎）；`--verify` 雙對照——筆畫數按一級字每 12 字取 1（291 字）、首筆按一級字 3,500 全量，白名單
  `assets/stroke/residual-whitelist.tsv`（7 字）與首筆 `assets/stroke/residual-first-strokes.tsv`（408 字，對照源幾何假陽性；首筆不按幾何近似放行，不符的字都要逐字進白名單）各自口徑之外一處不符即退出碼非 0；兩張對照表（筆畫數 / 首筆幾何類別）由 `mmh-reference` 子命令從 hanzi-writer-data 生成到 `data/mmh/`（Arphic 許可，不進倉庫、不隨包；缺席時跳過對照並提示）。
  來源、許可與驗收記錄見 `assets/stroke/README.md`。
- `pack codes`：把 `codes/stroke.tsv` 與詞庫（預設 `data/generated/dict.qj`）算成隨包原生碼錶 `data/generated/codes/stroke.qj`
  （鍵位 1→h 橫 / 2→s 豎 / 3→p 撇 / 5→z 折 / 點捺 n→n；單字「前 4 筆 + 末筆」，不足 5 筆按實際取；片語每字首筆，
  二字 2 碼、三字 3 碼、四字以上 = 前三字首筆 + 末字首筆；詞裡有一個字不在筆畫表裡整詞跳過）；
  `--stroke` / `--dict` / `--output` 改路徑，後設資料預設「筆畫」/ `OFL-1.1` / CNS11643 數位發展部署名（可覆蓋），資料版本取筆畫表日期。
  2026-09-20 實測：詞庫 92,821 詞條 / 91,904 詞，有碼 91,773、無碼跳過 131，碼錶 91,773 條 / 3.0 MB。

## apps/linux

`qingjian-linux-server` 為獨立產品 `0.1.0-dev`，裝配本地 Engine、詞庫、釋義、頻率學習、個人 n-gram、詞彙記錄與可選輸入日誌，
本地整句模型優先載入使用者 `~/.local/share/qingjian/models/hanzhang-tongbian/` 或隨包 `data/models/hanzhang-tongbian/`，缺失時回退 `models/hanzhang-zhiwei/`；舊使用者目錄相容讀取。按 `[model] enabled` 在後臺載入、停鍵 80 ms 後重排，節拍與 Windows Server 的 `dispatch/rescore` 相同；不接雲服務。`dispatch/session` 交換每個上下文的 EngineSession；真正的能力變化丟棄輸入，普通焦點切換隔離儲存。
預設面板外掛僅轉換事件，Shift 模式、候選點選、分頁和失焦提交都由 Server 決定。

Unix socket 用共享長度字首與 Frame（當前公共版本 8，與 `PROTOCOL_VERSION` 同步，Fcitx5 外掛裡寫死在 `qingjian.cpp` 的 OpenSession）；外掛複用一條連線，每個上下文獨立會話。Linux v3 擴充套件逐會話握手、確認 Sensitive/Password/Disable 後接受按下/釋放、焦點和點選事實。
候選回報繫結連線代次、上下文和服務端幀序號，僅當前聚焦頁的有效釋義進入 `note_displayed`，不把生成幀算作已展示。
`[general] preedit` 使用已有 `both` / `inline` / `window`；沒有新增 Linux 自繪配置。詳見 [linux-fcitx5.md](linux-fcitx5.md)。
