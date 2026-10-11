# 同類項目與資料來源

## 水杉輸入法（MetasequoiaIME）

- https://github.com/metasequoiaime/MetasequoiaImeTsf
- Windows 專用，純 TSF，自研引擎（非 Rime），GPL-3.0。截至 2026-09 約 950 star，近期新增 5000+ 使用者。
- 已經實現了「候選旁顯示譯文」：豎排候選窗中為候選項顯示中文與**所選語種**的互譯，每項最多兩條簡短釋義；
  本地詞典優先，未命中可走騰訊雲 TMT 或自建 DeepLX 相容服務。
- 另有日文羅馬字輸入模式、中英混輸。
- issue 裡翻譯功能相關的反饋佔比不低（錯譯、想直接上屏譯文、日語學習模式加羅馬音、短語翻譯不準），
  說明使用者在實際使用這個功能。

對青簡的意義：

- 「打字時看到譯文」這個互動形態已經被水杉驗證，不需要再做原型驗證。
- 青簡相對水杉的差異只剩三條：非 Windows 平臺（水杉沒有 macOS / Linux）、Rust 平臺無關 Core、
  更剋制的單條 annotation（水杉最多兩條，可走雲端）。
- 青簡的輸入法本體需要先達到「不比水杉差」這條線，翻譯功能才有意義。
- 水杉的 TSF 程式碼可以作為 Phase 5 的參考，但它是 GPL-3.0，除非青簡也選 GPL，否則不能搬。
- 水杉的程序結構（TSF DLL + 獨立 Server 程序 + UI）是 Windows 平臺層應採用的結構。

## Rime

- https://rime.im/
- librime（C++，約 5 萬行）+ 各平臺前端：Squirrel（macOS）、Weasel（Windows）、ibus-rime / fcitx-rime（Linux）。
- 青簡的「Core + 薄殼」架構與 Rime 相同，Rime 證明了這個結構能跑十幾年。
- Rime 候選自帶 comment 欄位，Squirrel 顯示在候選旁，配合 librime-lua 也能做出類似的譯文標註。

## 資料來源

| 用途 | 來源 | 許可 | 備註 |
|---|---|---|---|
| 拼音詞庫（產品） | 自建，源資料在 `assets/lexicon/`：《通用規範漢字表》8105 字（政府公佈的規範字表，iDvel 轉錄並與 shengdoushi 分級表核對）、《現代漢語常用詞表（草案）》56k 詞（教育部 2008，liuxilu 校對版，自帶拼音）、THUOCL 11 類領域詞 15.7 萬（2026-09-06 起按語料次數拆開：常見的留基礎詞庫，其餘各成一本隨包 `.qj`；清華 NLP，MIT）；2026-09-27 起補入 jieba 分詞詞表（MIT）對照出的缺失高頻詞（讀音逐字取 Unihan，詞頻按與原語料統計的迴歸關係折算，見 `assets/lexicon/domain_words.tsv` 頭注） | 字表與常用詞表是官方檔案的轉錄，轉錄倉庫未附許可證；THUOCL MIT；讀音 Unihan（Unicode License v3） | `dict-convert lexicon` 生成 `dict.tsv`（20.5 萬條，隨倉庫放 `assets/lexicon/dict.tsv`）：字與領域詞讀音來自 Unihan，5.6 萬含多音字的領域詞和 1.2 萬含多音字的常用詞由 LLM 標註再逐字對照 Unihan 校驗（常用詞表自帶拼音對多音字錯了 288 處：部長 chang、西藏 cang、成都 dou……，以標註為主讀音、原讀音降權保留），詞頻來自下面兩份語料的統計（兩遍：先按底值分詞統計，再用真實詞頻重統計）。2026-09-05 起取代霧凇拼音（GPL-3.0 only，與本專案不相容，已從資料目錄與工具裡移除） |
| 英文詞表（產品） | 同一資料包 `05_english`：ESDB（en-wl/wordlist，MIT-like）、CSpell software-terms（MIT）、typos 誤拼對照（MIT） | 均寬鬆，許可原文隨包 | `dict-convert english` 取純字母詞 9.5 萬，詞頻 wordfreq；誤拼對照表以後可喂英文糾正。取代霧凇 en_dicts |
| 語言模型語料 | 中文維基（HF `wikimedia/wikipedia` 20231101.zh） | CC BY-SA 4.0 | 百科體，第一人稱、口語詞幾乎沒有（去 / 累 這類詞計數極低），單獨用它整句會偏向地名術語；由它統計的 bigram 表按 CC BY-SA 對待 |
| 語言模型語料 | LCCC（清華 `thu-coai/lccc`，微博對話） | MIT | 口語對話，補維基缺的日常用語；兩份語料合併統計。`tools/corpus/parquet_to_text.py` 轉文本，`dict-convert bigram` 統計 |
| 語言模型 | Rime 八股文 / octagram | essay 只是詞頻表；octagram 是 LGPL | 未採用：essay 沒有二元資訊，octagram 檔案格式是 Rime 私有的 |
| 中→英 / 中→日 | `tools/gloss-gen` 用 LLM（DeepSeek）離線批次生成，資料在 `assets/glossary/` | 模型輸出，許可乾淨 | 詞性 + 譯詞 + 日文假名；2026-09-05 對整個自建詞庫跑了全量：23.9 萬詞，詞庫多字詞 91% 有英文釋義、98% 有日文釋義，單字 92%；17.6 萬詞約 100 分鐘、幾十元。有道等網頁詞典介面不用：逆向的簽名介面不穩，攢下來的結果再分發是侵權 |
| 中→英（備用） | CC-CEDICT | CC BY-SA 4.0 | 需署名；share-alike 只約束資料本身及其加工版，不傳染程式碼。**沒有詞性**、釋義偏長，已被 LLM 表取代，`dict-convert cedict` 仍能生成 |
| 日文讀音 / 詞性（候選） | JMdict（EDRDG） | CC BY-SA 4.0 | 以後校對 LLM 給的假名與日文詞性 |
| 詞彙等級（統計） | 已用：CEFR-J Wordlist 1.5（A1–B2，免費須署名）+ Octanove C1/C2（CC BY-SA 4.0）；JLPT N5–N1（Tanos CC BY，經 elzup MIT 整理）。四六級 / 商務英語詞表在 GitHub 上只有大綱詞彙的轉錄、許可不明，沒用 | 已核 | 「統計」頁按級數見過 / 看熟 / 上屏過的譯詞（`assets/levels/`）；也可做英→中 / 日→中方向的種子；不進候選視窗 |
| emoji | Unicode CLDR 中文與英文 annotations（`cldr-json` 的 `annotations/{zh,en}`、`annotationsDerived/{zh,en}`） | Unicode License v3（寬鬆，需保留版權宣告） | `dict-convert emoji --language zh\|en` → `assets/emoji/emoji-{zh,en}.tsv`，隨倉庫與釋出包提供；許可文本在 `assets/emoji/LICENSE-unicode.txt`。日文表要等有日語輸入模式 |
| 英文詞頻 | wordfreq（Python 包） | 程式碼 MIT，資料 CC-BY-SA 等 | 只取每個詞的 Zipf 頻率數字給英文補全排序，`tools/corpus/english_frequency.py` 生成 `english-frequency.tsv`（gitignore） |
| 形碼碼錶（五筆） | [sxjudya/rime-wubi86-jidian](https://github.com/sxjudya/rime-wubi86-jidian) 的 `wubi86_jidian.dict.yaml`（86 五筆極點碼錶） | Apache-2.0 | `dict-convert wubi` → `assets/wubi/wubi86.tsv`（8.9 萬條）。**碼錶自帶的權重不用**（那是碼錶順序不是語料詞頻），詞頻按詞面從青簡詞庫回填，與拼音方案同一把尺子。上游另有 `_extra` / `_extra_district` 兩張擴充套件表，暫未併入 |

## 許可策略

- 程式碼：GPL-3.0-or-later（2026-09-07 定，此前測試階段是「保留所有權利」）。選強 copyleft 是為了堵閉源抄走，與鼠須管 / 水杉一致；名字與 logo 不授權。
  `LICENSE`、「關於」頁（`about.rs` 的 `LICENSE_NOTE`）、README 三處保持一致；pkg 裡的 `license.txt` 從 `LICENSE` 拷。
- 產品資料的原始檔進 `assets/`（`assets/lexicon/` 詞庫源、`assets/emoji/`），帶各自的許可證與署名檔案；
  語料、Unihan 這類體積大或可重新下載的中間輸入放倉庫根目錄 `data/`（gitignore），生成物在 `data/generated/`。
  許可與本專案不相容的資料（霧凇拼音）已不再使用，連開發測試也不用，免得產品資料被它汙染。
- 資料檔案不併入程式碼許可：每個資料來源在 `assets/` 下帶自己的 LICENSE 和署名檔案，README 裡列出。
  CC BY 只需署名；CC BY-SA 的 share-alike 只約束資料本身及其修改版，不傳染到程式碼，
  但對 CC-CEDICT 做的任何加工（裁剪、合併 gloss）都要以 CC BY-SA 釋出。
- 每新增一個數據源或依賴，同步更新上表和 README 的 License 一節。
- GPL 專案（水杉、部分 Rime 前端）的程式碼只能讀不能搬。

## 隨包資料清單

產品包裡帶的資料與各自的許可（偏好設定「關於」頁列的就是這份）：

| 資料 | 來源與許可 |
|---|---|
| 詞庫 | 通用規範漢字表；現代漢語常用詞表（liuxilu 校對版）；THUOCL 領域詞（清華大學自然語言處理實驗室，MIT）；jieba 分詞詞表（fxsjy/jieba，MIT，2026-09-27 起用於缺失高頻詞對照與詞頻折算）；讀音取自 Unihan（Unicode License v3） |
| 語言模型 | 中文維基百科（CC BY-SA 4.0）與 LCCC（清華大學 CoAI，MIT）語料統計 |
| 釋義表 | 由大語言模型（DeepSeek）離線生成，青簡自建 |
| emoji | Unicode CLDR annotations（Unicode License v3） |
| 英文詞表 | ESDB / SCOWL（© Kevin Atkinson，按其許可保留版權宣告）；CSpell 詞典（MIT） |
| 詞彙等級 | The CEFR-J Wordlist Version 1.5（Yukio Tono，Tokyo University of Foreign Studies，[cefr-j.org](http://www.cefr-j.org/download.html)）；Octanove Vocabulary Profile C1/C2（CC BY-SA 4.0）；JLPT 詞表（[tanos.co.uk](http://www.tanos.co.uk/jlpt/)，CC BY；經 elzup/jlpt-word-list 整理，MIT） |
| 五筆碼錶 | 86 五筆極點碼錶（[sxjudya/rime-wubi86-jidian](https://github.com/sxjudya/rime-wubi86-jidian)，Apache-2.0）；編碼來自上游，詞頻由青簡詞庫按詞面回填 |

各許可證原文在 `assets/` 對應目錄下。霧凇拼音（GPL）已徹底移除，不要再引入。
