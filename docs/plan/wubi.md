# 五筆（形碼）支援方案

2026-09-15 定。第一期做 **86 版**，平臺範圍 **Core + CLI + Windows**，配置收斂到 `[general] scheme` 並把配置遷移一起做。
本文記錄技術判斷與分期；實現要點落地後同步 `docs/notes/crate-notes.md`。

## 一、為什麼五筆不能按雙拼的方式接

現有程式碼裡「輸入方案」的抽象是**「把按鍵解碼成拼音串」**，雙拼與注音共用的是**拼音**這個底層，不是**方案**這個抽象：

```rust
// crates/qingjian-core/src/engine/setup.rs:79
pub(super) fn decode(&self, keys: &str) -> Option<EngineDecoded> {
    if self.zhuyin { Some(EngineDecoded::Zhuyin(crate::zhuyin::decode(keys))) }
    else { self.shuangpin.map(|s| EngineDecoded::Shuangpin(s.decode(keys))) }
}
```

解出來的 `EngineDecoded`（`engine/decoded.rs`）提供 `pinyin / tail / segmentation / marked / keys_for`，
隨後原樣進 `engine/query/mod.rs:79` 之後的詞級查詢、排序、整句。

五筆在這一層就不成立：碼串不是拼音，切不出音節，沒有讀音，沒有整句。拼音耦合在三個層次上：

| 層 | 耦合點 | 五筆的情況 |
|---|---|---|
| 解析 | `parser::segment`（`parser/mod.rs:54`）+ `SYLLABLES` 表（`parser/syllable.rs:7`） | 碼串無需切分，按音節切分本身就是錯的 |
| 詞庫 | `Dictionary` 的鍵是空格分隔的拼音串，查詢介面全部收 `SyllablePattern`（`dictionary.rs:350-393`） | 需要按「碼」索引，字首匹配而非音節位置匹配 |
| 排序 | `ranking/scored.rs` 的 `full_last` / `coverage` / `abbreviated` / 模糊音 `penalty` | 碼等長時這些欄位全部退化，詞頻主導 |

所以「在 `shuangpin/` 旁邊加個 `wubi/` 目錄」不成立。

## 二、方案：兩條平行管線

把方案抽象提升一層，**Engine 在 `query_inner` 頂部分派到兩條平行管線**，而不是讓五筆偽裝成拼音：

```rust
// engine/query/mod.rs:62 query_inner 的開頭
if let Some(code) = self.code {
    return Ok(self.query_code(keys, rest, start));  // 形碼：碼串 → 碼錶 → 候選
}
// 以下原有拼音管線不動
```

三條理由：

1. **現成的先例。** `query/mod.rs:103-129` 那個「第一個字母都切不動」的分支已經在返回
   `segmentations: Vec::new()` + `tail: keys` 的 `Query`，下游（`Engine::annotate`、輸入日誌、
   `note_displayed`、各殼渲染）本來就吃得下「無切分」的 `Query`。五筆複用這個形狀，不新造結果型別。
2. **不該開的開關變成天然不存在**，而不是散落一地的 `if wubi`。沿用現有套路
   （`engine/correcting.rs:9` 對雙拼直接返回 `None`、`engine/setup.rs:70` 的 `modes()` 返回 `ModeKeys::LETTERLESS`），
   在一個集中判斷裡關掉：整句 / Viterbi、神經重排、模糊音、拼寫糾錯與詞圖敲錯邊、中英混輸、簡拼、
   v / u / i 字首快捷鍵（**`v` 與 `i` 在五筆裡是字根鍵，必須讓位**）。
3. **青簡的產品特色全部自動保留。** `Learner`（詞頻 / 使用者詞 / 選擇學習）、`Translator` + `Glossary` 釋義、
   生詞本 `VocabularyBook`、CEFR / JLPT 等級、`InputLog`、`UsageStats` 都按「上屏的詞」工作，與輸入方案無關。
   **五筆使用者照樣有候選旁的譯文與生詞統計**——這是「Core 才是青簡」換來的好處，也是這個功能的賣點。

### 資料層：靜態全量碼錶，不做執行時取碼

新增 `CodeTable`（`crates/qingjian-dictionary/src/code_table.rs`，與 `Dictionary` 同屬「文本 → 鍵」的查表；
只有一個檔案、沒有同詞幹的兄弟，按約定不另開目錄）。詞目與 `WordList` 一樣是排好序的 `Vec`，
查詢靠**字首二分**——比現在按音節位置逐級收窄簡單。`.qj` 容器加 `Kind::Code`（`crates/qingjian-format`，階段 2）。

片語取碼規則（二字詞 2+2、三字詞 1+1+1、四字及以上 1+1+1+1）**在生成資料時算好，執行時不算**：
規則連帶簡碼與識別碼很繞，而靜態碼錶夠小也夠全；碼錶沒有的詞靠使用者詞學習補，不做執行時推導。

一級 / 二級 / 三級簡碼不專門實現——它們就是「字首查詢 + 詞頻排序」的自然結果（`g` → 一，`gg` → 該字首下詞頻最高的字）。

### 使用者詞

現有自動造詞（連著選出的兩個詞合成使用者詞）按「詞的字數 = 音節數」判斷，形碼沒有音節，自然不觸發
（`try_auto_word` / `finish_buffer` 裡 `chars != syllables.len()` 直接返回），不必另加開關。
「同一個碼下選過的詞靠前」由現有的 `Learner::record_choice` 負責，形碼不需要新的使用者詞表。

### 階段 1 落定的三件事（2026-09-15）

1. **候選是新的一種 `CandidateKind::Code`**，不是塞進 `Chinese`。它的 `syllables` 為空、上屏吃掉整段作用域
   （`commit` 裡的 `whole_scope`）。這樣「沒有音節」是型別裡的事實，而不是靠空串矇混；整段造詞也隨之不觸發。
2. **簡碼不用專門做**。一級 / 二級 / 三級簡碼就是「編碼打全的詞排在同字首的長編碼詞前面」，
   由 `CodeTable::lookup` 的 `exact` 位給出，與拼音側「音節數與輸入一致者優先」是同一個排序位。
3. **碼錶第一期只讀 TSV**，不進 `.qj` 容器（`Kind::Code` 留到階段 2 與 `pack` 一起做）。
   段 1 的 CLI 入口是 `--wubi <碼錶.tsv>`。

## 三、資料

| 來源 | 許可 | 結論 |
|---|---|---|
| [rime-wubi86-jidian](https://github.com/sxjudya/rime-wubi86-jidian)（極點 86 碼錶） | Apache-2.0 | **採用**：寬鬆、詞量最大 |
| [rime/rime-wubi](https://github.com/rime/rime-wubi) 官方 `wubi86.dict.yaml` | LGPL-3.0 | 備選：與本倉庫 GPL-3.0-or-later 相容，但要單獨帶 LICENSE |
| Gitee「大一統五筆」 | 未宣告 | 不能用 |

**詞頻不用碼錶自帶的權重**（那是碼錶順序，不是語料詞頻）。用現有 `assets/lexicon/dict.tsv` 裡
同一個詞的語料詞頻交叉回填——換方案不會讓同一個詞的排序變奇怪，釋義兜底與生詞識別也對得上。

管線：`tools/dict-convert` 加 `wubi` 子命令讀碼錶 → `assets/wubi/wubi86.tsv`（`詞\t碼\t詞頻`）
→ `pack code` → `code-wubi86.qj`。

`qingjian-dictionary/src/import/rime.rs` 已能把 Rime `.dict.yaml` 的 `詞\t拼音\t權重` 轉成青簡 TSV，
五筆碼錶正好是這個格式（第二列是 `ggll` 這樣的碼），轉換主體可以直接複用。

**許可與署名**（同步四處，見 `docs/design/landscape.md:55`）：`assets/wubi/LICENSE`、
`docs/design/landscape.md` 的資料來源表與隨包資料清單、README 的 License 一節、偏好設定「關於」頁。

## 四、配置與殼

現狀 `[general] shuangpin = ""` + `[general] zhuyin = false` 是**兩個欄位表達同一個維度**，
再加一個並列欄位就是三個欄位打架。**收斂成 `[general] scheme`**：

```toml
[general]
scheme = "pinyin"   # pinyin | xiaohe | ziranma | microsoft | sogou | zhuyin | wubi86
```

值就是現有 `ShuangpinScheme::key()`（`shuangpin/scheme.rs:32`）與 `zhuyin` 的統一。
舊鍵讀取時相容、寫入時遷移，落地時把 `docs/plan/todo.md` 裡掛著的「配置檔案版本遷移」一起做掉。

| 殼 | 落點 |
|---|---|
| Core | `engine/mod.rs` 的 `shuangpin` 與 `zhuyin` 收斂為一個欄位；`engine/setup.rs:20-33` 的 setter 合併；階段 1 已加的 `code: Option<CodeTable>` 改為隨 `scheme` 裝載（哪個碼錶由配置決定，Core 不需要 `CodeScheme` 列舉） |
| 配置 | `crates/qingjian-platform/src/config/general.rs:50-97` |
| CLI | `apps/cli/src/args.rs:76` 加 `--wubi`；`apps/cli/src/main.rs:236-250` 應用；回放 `apps/cli/src/replay/mod.rs:95-116` 認新 scheme |
| Windows 設定 | `apps/windows/settings/src/panel/pages/general.rs:14-20` 的 `SHUANGPIN` 列表擴成「輸入方案」；`panel/message.rs:12-13`、`panel/component.rs:41-44` |
| Windows Server | `server/src/main.rs:188-189` 啟動應用；`dispatch/reload/mod.rs:107-108` 熱載入；狀態條 `dispatch/status/mod.rs:93-99` 用 `scheme.label()` |
| macOS | **本期不動**。順帶記一筆：`apps/macos` 至今沒有接注音（全目錄無 `set_zhuyin_mode` 引用），mac 接方案時要把注音一起補上 |

中英切換仍然是布林，不變（架構約定「輸入方案是配置項，不是模式」）。

## 五、分期與驗收

| 階段 | 內容 | 估計 |
|---|---|---|
| 1 ✅ | Core `CodeTable` + `query_code` + Engine 分派 + CLI `--wubi`（2026-09-15 完成，233 + 21 個測試全綠，CLI 端到端驗過） | 3–5 天 |
| 2 | `dict-convert wubi` + 詞頻回填 + `pack code` + 許可署名 | 1–2 天 |
| | 　└ `dict-convert wubi` 與詞頻回填**已做**（2026-09-15，讀 Rime `.dict.yaml`，詞頻按詞面從青簡詞庫回填，4 個用例）；剩下載碼錶、`Kind::Code` 打包與許可署名 | |
| 3 | `[general] scheme` 收斂 + 配置遷移 + Windows 設定頁與狀態條 | 2–3 天 |
| | 　└ **已做**（2026-09-16）：`Scheme` 列舉與 `[general] scheme`；舊鍵 `shuangpin` / `zhuyin` 讀取時推斷（不自動改寫檔案）；CLI、macOS 偏好設定與 Windows 設定頁/Server/狀態條都改成讀寫新鍵；Windows Server 按方案裝載碼錶（使用者目錄優先、隨包 `assets/wubi/` 兜底，找不到只警告並按拼音跑）。打包與 macOS 接入**也已做**（2026-09-16 續）：`bundle.sh` 拷進 `Resources/wubi/`、Windows 安裝器拷進 `{app}\assets\wubi\`，兩處署名（macOS / Windows 的「關於」頁）都補了；macOS 的偏好設定那欄換成「輸入方案」下拉（`Scheme::ALL`），`apply_config` 裡一併設注音與碼錶——macOS 側因此順帶把一直沒接的**大千注音**也接上了。**還差**：macOS 與 Windows 的真機驗證 | |
| | 　└ 殼真帶上碼錶時，署名要同步：`bundle.sh` 是**按檔案白名單**拷資料的（`assets/wubi/` 現在一個都沒拷），偏好設定「關於」頁的 `ATTRIBUTIONS` 也是照隨包資料列的一份。現在兩處都沒加，因為 macOS 側還沒有輸入方案，包裡帶它只是白佔體積 | |
| 4 | 釋義 / 生詞 / 統計 / 日誌對齊驗證、輸入日誌 `scheme` 進回放、`docs/user/input/` 加頁 | 1–2 天 |
| | 　└ **已做**（2026-09-16）：回放按每條日誌的方案切換（`SchemeSwitcher`，形碼要有 `--wubi` 的碼錶）；釋義 / 生詞 / 詞彙記錄 / 輸入統計在形碼下照常，有測試釘住（`code_commits_still_feed_translations_vocabulary_and_usage`）；`docs/user/` 的按鍵表、拼寫糾錯、英文模式、偏好設定、模糊音與輸入方案各頁都補了五筆 | |

合計約 1.5–2 周（單人，不含真機來回）。

驗收沿用倉庫規矩：改動先跑 `apps/cli`。`--typing` 看逐鍵延遲（形碼每鍵應當比拼音更快，
沒有整句與模糊音的開銷）；`--replay` 看既有拼音日誌有沒有退化（應當完全不變）；
`--eval-text` 不適用於形碼，跑一次確認沒被誤觸發即可。

## 五之二、混輸（2026-09-16 追加，原計劃之外）

原計劃裡 `[general] scheme` 是**單選**的，選了五筆就完全不進拼音管線。實際用起來要的是
「雙拼和五筆分開和組合」，所以拆成**兩條獨立的軸**：

```toml
scheme = "pinyin"    # 拼音側：pinyin / xiaohe / ziranma / microsoft / sogou / zhuyin / none（關）
wubi   = "wubi86"    # 形碼側：空為關
```

三種用法：雙拼 alone（`xiaohe` + 空）、五筆 alone（`none` + `wubi86`）、**混輸**（兩邊都開）。
`Engine` 上對應兩個開關 `set_code_table` / `set_phonetic`，`query_inner` 按它們分派到
`query_code` / `query_phonetic` / `query_mixed`。

### 排序：編碼打全的形碼詞在前

2026-09-18 定：**編碼打全的形碼詞 → 拼音候選 → 只命中編碼字首的形碼詞**。一律形碼在前的話，
真實資料下 `kai` 的首選是編碼 `kaik` 的 中共黨員、`shi` 是 椒，拼音那半邊沒法用；打全的編碼是精確的，
留在最前（`ga` → 開、`wo` → 夥）。混輸下模式鍵同雙拼換成大寫（`V1+2`），四碼以內不做拼寫糾錯。

另外兩條規則也是拍板的：

- 拼音讀不出來時（`ggll` 切不成音節）整個按形碼走，兩邊都空才算錯。
- 五筆碼最長 4 位，**第 5 個字母起自然只剩拼音**，不用特判。

## 六、不做的事

- **執行時取碼推導**（由單字碼拼出片語碼）：規則太繞，靜態表夠用。
- **形碼下的整句轉換**：五筆的整句沒有意義，碼長即詞長。
- **臨時拼音 / 反查（`z` 萬能鍵）**：第一期不做，等有使用者反饋再說。
- **98 版與新世紀版**：`CodeTable` 按多碼錶設計，加版本只是加資料，但第一期只做 86。
