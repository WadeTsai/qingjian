# qingjian-cli

青簡 Core 的命令列殼：不依賴任何平臺 API，用來查候選、看排序、量效能、回放評測。改 Core 的任何東西都先在這裡驗證，再去碰輸入法。

```bash
cargo run --release -p qingjian-cli -- kaifa            # 查一個拼音
cargo run --release -p qingjian-cli -- kaifa nihao      # 查幾個
cargo run --release -p qingjian-cli                     # 互動模式
```

效能相關的（`--typing`、`--replay`）一律用 `--release`，debug 構建慢十倍以上，數字沒有意義。

## 資料檔案

預設按優先順序找：`data/generated/` 裡打包好的 `.qj` → 那裡的 TSV → 倉庫自帶的產品資料（`assets/lexicon/`、`assets/glossary/`）→ `assets/sample/` 的樣例。
語言模型 `data/generated/lm.qj`（或 `lm-unigram.tsv` / `lm-bigram.tsv`）有就載入，沒有就退化成一元詞頻整句；emoji 表在 `assets/emoji/`。

| 引數 | 作用 |
|---|---|
| `--dict <路徑>` | 主詞庫（`.qj` 或 TSV） |
| `--extra-dict <路徑>` | 附加詞庫，可給多個，與主詞庫一起查；領域詞庫在 `data/generated/dicts/*.qj`（輸入法預設只開 `idioms`，回放要對齊就帶上它） |
| `--glossary <路徑>` | 學習語言的釋義表 |
| `--language en\|ja\|es\|vi` | 學習語言（也可用環境變數 `QINGJIAN_LEARNING_LANGUAGE`） |
| `--english <路徑>` | 英文詞表（中英混輸、英文模式候選） |
| `--user-dict <路徑>` | 使用者詞頻檔案（`user.tsv`，同目錄的 `user-words.tsv` / `user-choices.tsv` / `user-ngram.tsv` / `user-english.tsv` 一起讀）；給了就在退出時寫回，不給則只在本次會話內學習 |
| `--config <路徑>` | 配置檔案，預設與輸入法共用 `~/Library/Application Support/Qingjian/config.toml`；測試時給一份 `[predict] enabled = false` 的，免得每次查詢都等雲端 |

## 行為開關

| 引數 | 作用 |
|---|---|
| `--fuzzy z-zh,n-l,…` / `--fuzzy all` | 模糊音，覆蓋配置裡的 `[fuzzy]` |
| `--shuangpin xiaohe\|ziranma\|microsoft\|sogou\|abc\|xiaolang\|shoudao\|off` | 雙拼方案，覆蓋配置；`off` 強制全拼 |
| `--english-mode` | 英文模式（輸入法裡是 Caps Lock 亮著）：字母不當拼音，候選來自英文詞表 |
| `--predict` | 強制開雲聯想並等結果列印；金鑰來自配置或環境變數 `QINGJIAN_API_KEY` |
| `--limit N` | 只顯示前 N 個候選（預設 9） |

## 輸出

每個查詢列印：切分（多種切分用 `\|` 分開，`…` 表示簡拼 / 殘缺音節）、糾錯（`糾正: nihooma → ni hao ma`）、候選列表（序號、候選、`[句]` 是整句候選、右側詞性與譯文、雲端詞帶 ☁）、各階段耗時。

## 互動模式

不給拼音引數就進入互動模式：

| 輸入 | 作用 |
|---|---|
| 拼音 | 查候選 |
| 序號 | 上屏上一次查詢的第 N 個候選並記入學習 |
| `:raw` | 上一次輸入原樣上屏（相當於回車） |
| `:del N` | 刪掉第 N 個候選：使用者詞整個刪，詞庫詞清掉對它的學習 |
| `:q` | 退出 |

## 效能：`--typing`

把每個輸入當作一鍵一鍵敲進去，每個字首都查一次，列印每鍵各階段耗時與最慢 / 平均值。目標每鍵 10 ms 以內。

```bash
cargo run --release -p qingjian-cli -- --typing jintianwanshangwomenquchifan
```

## 回放評測：`--replay`

讀輸入法記的輸入日誌（`~/Library/Application Support/Qingjian/input-log.jsonl`，每次上屏一行），把每行當時的鍵重新餵給引擎，看現在的排序會不會把當時選的詞放在首選：

```bash
cargo run --release -p qingjian-cli -- --replay ~/Library/Application\ Support/Qingjian/input-log.jsonl --misses 20
```

- 按來源（詞 / 整句 / 英文 / 其他）算首選命中率、前五命中率、平均名次、不在候選裡的條數；雲端詞、雲端整句、原樣上屏、譯詞不評分只計數。
- 只在記憶體裡學習，不寫任何檔案；按日誌順序回放，命中的候選照樣上屏讓上文往前走。
- 加 `--user-dict …/user.tsv` 就帶上現有的學習資料（回放分支不落盤）。不帶是「冷引擎」，量的是詞庫與模型本身；帶上量的是實際體驗，兩者之差是學習功能的貢獻。
- `--misses N` 列印前 N 條沒命中首選的例子（作用域、當時選的、現在的前三）。

排序、整句、糾錯的任何改動，改前改後各跑一次比差值。

## 日誌

終端裡日誌走 stderr，`RUST_LOG=debug` 看逐鍵細節；設定 `QINGJIAN_LOG_DIR` 時再按天寫檔案。
