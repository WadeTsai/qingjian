# 主題（預覽版）

候選視窗與 Windows 懸浮狀態條由青簡渲染器（`crates/qingjian-render`）按主題畫成點陣圖，mac 與 Windows 用同一個渲染器、同一份主題，畫出來逐畫素一致。
這裡記格式全貌與實現，寫給維護的人；使用者怎麼寫主題見 `docs/user/settings/themes.md`。本文只寫已經做到的，做不到的列在末尾「預覽版的限制」。

## 主題檔案

- 一個主題一個目錄：`theme.json`，加上它引用的 `images/…`（PNG、SVG）與 `fonts/…`（TTF / OTF / TTC）。包內路徑一律相對 `theme.json`，拒絕 `..` 與絕對路徑。
- 內建主題四個，隨 crate 編進來（`crates/qingjian-render/themes/<id>/theme.json`）：`qingjian` 青簡綠（預設）、`system-blue` 系統藍、`wechat` 微信綠、`sakura` 櫻花；系統藍、微信綠 `extends` 青簡綠、只改顏色變數，櫻花整套重寫了視窗、元件與狀態條（只有淺色）。
  內建主題的圖片用 `include_bytes!` 編程序序（`theme/mod.rs` 的 `BUILTINS` 裡每個主題一張「路徑 → 內容」表，與使用者主題走同一套解碼），不帶字型檔案、只用系統字型；測試檢查內建主題用到的圖片都編進去了。使用者主題 `extends` 內建主題時，主題目錄裡沒有的圖片到那個內建主題編程序序的檔案裡找（只看頂層 `extends` 那一層）。
- 使用者主題放在 `<使用者資料目錄>/themes/<id>/`，由 `ThemeLibrary` 與內建主題合成一個列表（內建在前、使用者按 id 排）。目錄名必須等於 `meta.id`，不能與內建主題重名，
  不能是 `system` / `light` / `dark`；讀不進來的跳過並記警告。設定介面（mac 偏好設定、Windows 設定程式「候選視窗」頁）按顯示名列出，寫回 `[general] theme` 的 id。

### 為什麼用 JSON

主題是巢狀很深的圖層樹：TOML 寫深層巢狀要麼是一長串表頭、要麼是不能換行的內聯表；YAML 縮排錯一格結構就變，還有隱式型別轉換；自定義格式的解析器、高亮、補全都得自己做。
設定是平鋪的鍵值，仍用 TOML（`config.toml`）。

手寫時可以加註釋（`//`、`/* */`）與尾逗號（JSONC，同 VS Code 的 `settings.json`）：`theme/jsonc.rs` 在解析前把它們換成空格、保留換行，報錯的行號、列號對得上原檔案。

## 格式（schema 1）

內建主題全部用下面的寫法寫成，與改造前寫死的排版逐畫素一致（快照測試）。

頂層：

| 鍵 | 內容 |
|---|---|
| `extends` | 以某個內建主題為底：物件逐鍵合併，陣列與標量整個替換；目前只能繼承內建主題。不寫就以青簡綠為底，繼承鏈上每一層都一樣（內建主題也是），只有青簡綠自己是最底層 |
| `schema` | 格式版本，整數，只在不相容改動時加一；比渲染器認得的新時記警告、按認得的部分畫 |
| `meta` | `id`、`name`、`author`、`license`；`appearance: "light" \| "dark"` 鎖定外觀（見下） |
| `variables` | 顏色變數，`"#rrggbb[aa]"` 或 `{ "light", "dark" }`，節點裡用 `@名字` 引用 |
| `text` | `gamma`（可分深淺）、`family`（預設字族）、`styles`（命名樣式：`size`、`line_height`、`weight`、`family`、`style`） |
| `fonts` | 隨主題帶的字型檔案 `[{ "file": "fonts/x.ttf" }]` |
| `components` | 可複用的節點，`use` / `repeat` 按名字引用 |
| `windows` | `vertical` / `horizontal` 兩個根節點，按使用者選的排布取一個 |
| `status` | 懸浮狀態條（只有 Windows 有）：字型、內邊距、圓角、各處顏色、`effects`，排法固定；寫了 `root` 就按節點樹畫（見「狀態條」） |

節點：

| `type` | 屬性 |
|---|---|
| `frame` | `direction`（row / column）、`fill`、`border`、`radius`、`children`；帶 `table: { row_height }` 時排成表格 |
| `text` | `bind` 或 `text`（固定文字）、`font`、`color`、`stroke` |
| `icon` | `icon`（cloud / gear）、`size`、`color`；盒子預設與圖示同大，圖示垂直居中 |
| `preedit` | `font`、`typed` / `rest` / `struck` 三種拼音顏色、`caret: { width, color }`、`stroke` |
| `annotation` | `bind`、`font`、`gloss` / `fresh` / `faint` 三種顏色（`pos`、`separator` 可單獨寫，預設同 `faint`）、`tones`（只畫哪幾種片段）、`senses`（只畫前幾個義項）、`stroke` |
| `use` | `component`：引用元件，這裡寫的盒子屬性蓋過元件根節點的 |
| `repeat` | `bind`（`candidates`）、`component`：每項候選例項化一份 |

所有節點都可寫：`when`（顯示條件）、`id` + `transition`（過渡）、`animation`（迴圈動畫）、`margin` / `padding`（一個數或 `[上, 右, 下, 左]`，外邊距可寫 `"auto"`）、`gap`、
`width` / `height` / `min_width`、`position: "absolute"` + `inset`、`align_self`（start / end / center / stretch）、`span: "row"`（表格裡橫跨整行）、`opacity`（整棵子樹）、`effects`。長度單位是點。

### 資料

- **條件**：`when` 是資料欄位名，`!` 取反，`a|b` 任一成立。候選項裡：`highlighted`、`cloud`、`annotation`、`first`、`last`；
  整幀：`preedit`、`trailing`、`trailing.cloud`、`page`、`candidates`、`annotations`（任一候選有譯文）、`highlighted`、`highlighted.annotation`；
  輸入狀態：`mode.english`、`mode.traditional`、`mode.full_width`、`mode.scheme`（有方案名）。
- **繫結**：文字 `index`、`text`（候選項）、`page`、`trailing.text`、`mode.scheme`（方案顯示名）；譯文 `annotation`（候選項）、`highlighted.annotation`；列表 `candidates`。
- **條件顏色**：`{ "if": "highlighted", "then": "@hl_text", "else": "@text" }`，條件寫法同 `when`。系統藍、微信綠的高亮白字靠它；青簡綠裡 `hl_*` 與普通顏色同值。
- **譯文片段**：殼把譯文拼成一串帶種類（`Tone`）的片段：`gloss` 譯文、`fresh` 生詞譯文、`pos` 詞性、`separator` 義項間的 ` · `、`faint` 假名注音、`code` 輔碼。
  `tones` 按種類挑、`senses` 按分隔數截前幾個義項，詞性與譯文分兩行排就是兩個節點；挑完是空的也留著節點，主題給它定高就照樣佔行。系統繪製的舊路徑把 `pos` / `separator` 畫成與 `faint` 同色。
- **輸入狀態**（`Frame::mode`）：渲染器只給事實（三個布林與方案名），顯示什麼字由主題用 `when` 分支寫，渲染器裡沒有「中 / 英」這類文案。
  mac 殼組裝候選幀時填（中英看 Caps Lock，與選單欄一致；簡繁、標點、方案看配置；英文模式標點算半形）；
  Windows 由 Router 現算（與狀態條同一來源），經 `CandidateSink::show` 交給 UI 執行緒，不進 Server ↔ DLL 協議。方案名 `qingjian_platform::scheme_name`：同狀態條的 `scheme_label`，單開全拼時也寫「全拼」。
- **設定裡的字號**：`[general] candidate_font_size` / `annotation_font_size`（0 用主題的，不限範圍）由殼經 `Theme::with_text_sizes` 蓋到主題上：
  `candidate` 與 `annotation` 換成設定的字號，行高同比例；`index`、`preedit` 與表格 `row_height` 按候選字的比例縮放（`theme/text_sizes.rs`）。
  表格行高是下限（`minmax(row_height, auto)`），內容更高時撐開。
  狀態條不逐個樣式改字號：`render_status` 去掉設定的字號，把整條的倍數乘上候選字的比例，文字、圖示、邊距、點選邊界一起縮放。
- **狀態條**：`status.root` 是一棵與候選視窗同寫法的節點樹，`repeat` 繫結 `cells` 展開格子（Windows 現在是模式、標點、齒輪三格）。
  格子裡的條件 `emphasized`（當前模式、生效的全形標點）、`gear`、`first`、`last`，文字繫結 `text`；整條能用 `mode.*`（`Renderer::render_status` 收一份 `Mode`，Windows 的 `StatusView` 帶著，與候選視窗同一個 `Router::input_mode`）。
  點選按格：每格第一個產出的節點的右邊界（根座標），沒畫出來的格同前一格，最後一格延到內容右邊；格子之間的裝飾算前一格（`renderer/status/tree.rs`）。
  沒寫 `root` 走原來的固定排法（內建主題都是），Windows 的狀態條快照不變。
- **表格**：`repeat` 出來的每份元件是一行，元件根節點的子節點依次是各列，每列取各行最寬，行高至少 `row_height`；末尾自動補一列吃掉剩餘寬度，
  `span: "row"` 的格子（高亮條）因此能橫跨整個表格。列間距用各格的外邊距寫。豎排三列對齊就靠它。
  一行裡的各格（不含 `span`）按各自第一段文字的基線對齊：佈局後把基線偏高的格子往下補上邊距再排一遍，差不到一畫素的不動（`scene/baseline.rs`）；
  所以格子的上邊距只能把它往下壓，不能讓它高過同行的基線。

### 填充與效果

- **填充**：顏色（含條件顏色）；`{ "linear": 角度, "stops": [...] }`（CSS 角度）；`{ "radial": [cx, cy], "stops": [...] }`（圓心按比例、半徑到最遠角）；
  色標是顏色（均分）或 `[顏色, 位置]`；`{ "image": "images/x.png", "slice": [上, 右, 下, 左], "scale": 2 }`（slice 為圖片畫素的九宮格切邊、不寫就拉伸，scale 為一個點對幾個圖片畫素）。
  `border: { width, color }` 畫在內側。圖片隨主題目錄載入（`Theme::from_dir`），PNG 邊長 ≤ 4096。
- **SVG**：路徑以 `.svg` 結尾的圖片用 resvg 解析（`theme/svg_image.rs`），單位當作點、不管 `scale`，`slice` 也按 SVG 單位寫。載入時解析一次（檔案 ≤ 4 MB、原始尺寸 ≤ 4096），
  畫的時候才柵格：拉伸按盒子的畫素尺寸直接柵格，九宮格按原始尺寸 × 倍數柵格再切，每張最多快取 8 種尺寸，之後與 PNG 同一條貼圖路徑（圖片框的離屏快取也一樣）。
  不讀外部檔案（`<image href>` 的字串解析器換成一律返回空，否則 usvg 會按路徑讀盤），不開 `raster-images`（內嵌點陣圖不解碼）、不開 `text`（`<text>` 不畫、不帶字型庫）。
- **效果**：`effects: [{ "type": "drop-shadow" | "inner-shadow", "x", "y", "blur", "spread", "color" }]`，按寫的順序畫。
  形狀取節點自己畫出來的 alpha（圓角框、九宮格圖片的透明邊、文字），不含子節點；自己不畫東西的容器（沒填充的框、譯文、拼音行）取子節點；`spread` 只對框生效。
  投影墊在節點底下、半透明填充會透出來；內陰影壓在填充上、子節點下。
  視窗陰影就是根節點的投影：渲染器按投影伸出的距離在點陣圖四周留邊，殼按內容區對齊游標，mac 面板關掉系統陰影（系統繪製退路仍用系統陰影）。狀態條的陰影寫在 `status.effects`。
- **伸出視窗的裝飾**：負 `inset` 的絕對定位節點（`"inset": [-24, -24, "auto", "auto"]`）。佈局後把所有自己畫東西的節點（連同投影、描邊）併成畫出範圍，點陣圖按它開，
  根節點的盒子是內容區（`Rendered::content_*`）；殼照舊按內容區對齊游標、夾進螢幕，裝飾可以伸到螢幕外。
  滑鼠整個穿透：mac 面板本來就 `ignoresMouseEvents`，Windows 候選窗加 `WS_EX_TRANSPARENT`。
- **鎖定外觀**：`meta.appearance` 寫了就不跟隨外觀設定切換；用淺色圖片做底的主題要鎖淺色。

### 文字

- **字重**：`weight` 寫 100–900 或 `thin` / `extralight` / `light` / `regular` / `medium` / `semibold` / `bold` / `extrabold` / `black`。
  mac 的 SF 是可變字型、蘋方是多字重集合；Windows 另載入 Segoe UI 與雅黑 / 正黑的粗細體檔案，缺的字重挑最近的一檔。
- **行內改樣式**：節點的 `font` 可以寫 `{ "base": "index", "size": 12, "weight": "semibold", "style": "italic" }` 在某個樣式上改幾項，只改字號時行高等比縮放；字族不能在這裡改，要換字族就定義命名樣式。
- **字族與斜體**：`text.family` 是預設回退鏈，樣式上 `family`（字串或陣列）覆蓋它，`style: "italic"` 斜體；`system` 是介面字型（使用者選的字型，沒選就是平臺介面字型）。
  載入主題時把預設鏈與各樣式的鏈（按樣式名排）去重成字族表（`theme/font/families.rs`），`FontSpec` / `TextStyle` 裡只存序號，仍可按值複製。
  渲染器換主題時給每條鏈挑第一個字型庫裡有的字族，一個都沒有用介面字型；字型裡缺的字由 cosmic-text 按 locale 回退。沒有斜體面的字型傾斜 14° 合成（漢字也是）。
- **字型載入**：字型庫不掃系統字型目錄（fontdb 全掃幾百毫秒、幾十 MB），所以主題寫到的系統字族由殼查檔案：`Renderer::load_theme_fonts(theme, family_files)`，
  mac 走 CoreText、Windows 走 DirectWrite（與使用者在設定裡選字型同一個查法），字型庫裡已有的字族不再查。隨主題帶的字型（單個 ≤ 32 MB）在換主題時先載入。
  同一個檔案只加載一次，不解除安裝。
- **描邊**：`text` / `annotation` / `preedit` 上寫 `stroke: { width, color }`（顏色可寫條件）。cosmic-text 取字形輪廓（`SwashCache::get_outline_commands`）拼成一條路徑，
  圓角接頭按兩倍寬描一次、字形壓在上面，露出的是向外的寬度，不佔排版寬度；點陣圖 emoji 沒有輪廓不描。文字陰影、發光就是文位元組點上的 `effects`（形狀含描邊）。

### 動畫

- **過渡按 `id` 配對**（同 Figma 的 Smart Animate 按圖層名配對）：新一幀裡帶 `id` 且寫了 `transition` 的節點，在上一幀找同 `id` 的節點，從它的位置、大小、不透明度插值到新的；顏色、字號不過渡。
  `repeat` 展開出同一個 `id` 的多份時按出現順序加序號（`word#0`、`word#1`）各自配對；只出現一次的不加，所以只在高亮行裡出現的高亮條換行後仍配得上。上一幀沒有同 `id` 的就直接出現。
- `transition: { "duration": 毫秒, "easing": … }`：`easing` 為 `linear`、`ease`、`ease-in`、`ease-out`、`ease-in-out`，或 `[x1, y1, x2, y2]`（同 CSS `cubic-bezier`）。
- **迴圈動畫**：`animation: { "duration", "easing", "loop", "keyframes": [{ "at": 0–1, "x", "y", "rotate", "scale", "opacity" }] }`。每個屬性只在寫了它的關鍵幀之間插值，
  開頭、結尾沒寫時按預設值補（同 CSS 缺 0% / 100%）；`loop: false` 播一輪停在最後。變換作用於整棵子樹、不影響佈局（同 CSS `transform`）。時鐘從窗口出現算起，打字過程中不重置。
- **執行方式**：`Renderer` 留住上一幀的場景樹與佈局；`Rendered::next_frame` 告訴殼多久後要下一幀（沒有動畫為 `None`），殼到點調 `tick`，只按插值重畫、不重建樹、不重排版。
  新資料到了，新內容立刻畫出，只有帶過渡的節點從舊位置出發，按鍵到候選出現的延遲不變。畫出範圍按動畫能到的最遠處算（過渡取起止的並，迴圈取最大平移、旋轉後的外接圓），動畫中途點陣圖不變大小。
  過渡 60 幀、迴圈動畫上限 30 幀；視窗隱藏時停下，沒有動畫時不開定時器。系統開了「減少動態效果」或設定裡關了 `animations`（`Theme::with_animations`）時過渡直接跳到終點、迴圈動畫停在第一幀。
  殼：mac 用主執行緒 `NSTimer`，Windows 在 UI 執行緒 `SetTimer`。
- **區域性重畫**：迴圈動畫用髒矩形（`renderer/partial.rs`）：動畫節點把樹序切成幾段，不動的各段畫進圖層快取，每幀只在動畫節點上一幀與這一幀佔的區域裡按原先後疊「段、動畫節點、段……」，
  與整張重畫只差半透明疊加的取整（測得最多 3 / 255）。動畫節點的上級有半透明容器、拿子節點當陰影形狀的容器或別的動畫節點，或同時有過渡在播時，退回整張重畫。
  效果遮罩與圖片填充的框按「節點 + 位置大小 + 畫布大小」快取（`scene/cache_key.rs`）；九宮格里原圖同色的格子按純色填。

## 實現

```
theme.json + Frame 資料
  → 例項化（renderer/build）：按 when 取捨、按 bind 填資料，use / repeat 展開元件
  → 佈局：Taffy（flexbox）算出每個節點的矩形，表格自己按列取最寬
  → 繪製：tiny-skia 按樹序畫，陰影在離屏遮罩上做；文字 cosmic-text 整形 + swash 柵格
  → 點陣圖 + 內容區 + 各候選 / 狀態條格子的命中區域
```

- 內部全用畫素：主題裡的點數進來先乘縮放倍數。文位元組點的盒子頂邊就是行框頂邊，字形在行高裡垂直居中。
- **容錯**：載入時檢查顏色變數、文字樣式、元件引用，找不到的記警告；渲染時顏色退回透明、樣式退回 16/19、元件不畫。不認識的鍵直接忽略。
  讀不進來的圖片不畫，讀不進來的字型按回退鏈往後找。正在用的主題整個讀不進來時用青簡綠。
- **熱載入**：目錄戳（各 `theme.json` 的修改時間與大小）變了就重讀。mac 在啟用期間每秒的配置檢查裡 `refresh()`；Windows Server 在熱載入輪詢裡比戳，變了把設定重發給 UI 執行緒重讀。儲存後約一秒生效。
- **配置鍵**：`[general] appearance` 是外觀（system / light / dark），`[general] theme` 是主題 id（預設 `qingjian`）。2026-09-18 之前外觀寫在 `theme` 裡：
  沒寫 `appearance` 且 `theme` 是這三個詞之一時按外觀讀、主題用內建的，所以這三個詞不能當主題 id。讀取一律走 `GeneralConfig::appearance()` / `theme_id()`。
  Server ↔ DLL 幀協議裡的欄位在 Rust 裡改名為 `appearance`，線上仍叫 `theme`，舊 DLL 照常能解析，協議版本不變。
- **效能**（2026-09-18，mac、release、2 倍屏、字形快取已熱）：內建主題新幀約 6.6 ms；能力展示主題（九宮格 + 十幾處陰影）新幀約 15.5 ms、過渡幀約 7.3 ms、
  迴圈動畫幀約 0.2 ms（第一幀切段約 7 ms）。

## 測試與預覽

- 快照測試 `crates/qingjian-render/tests/snapshots.rs`：樣例幀（`tests/scenes/`）按淺 / 深色畫成點陣圖，與 `tests/snapshots/<os>/` 的基準逐畫素比；字型環境與基準不一致時跳過。
- 能力展示主題 `crates/qingjian-render/tests/themes/showcase/`（程式生成的圖片）覆蓋九宮格、陰影、伸出裝飾、過渡與迴圈動畫。
- 主題功能 `tests/theme/`：
  - `fonts.rs`：系統字族經殼查檔案載入、回退鏈、隨主題帶的字型；
  - `status.rs`：狀態條節點樹按 `cells` 展開、點選邊界、`mode.*` 分支；
  - `svg.rs`：九宮格四角按倍數、超出主題目錄的路徑不讀；`theme/svg_image.rs` 裡測了不讀外部檔案。
- 離線預覽：`cargo run --release -p qingjian-render --example preview -- --theme <主題目錄>`，把樣例幀畫成 PNG；主題寫的系統字族在 `--font-dir` 裡按名字找。

## 預覽版的限制

- 狀態條的節點樹畫法不播動畫；mac 沒有懸浮狀態條（選單欄圖示），主題的狀態條只在 Windows 上顯示。
- 翻頁只能顯示頁碼文字；畫出來的翻頁按鈕點不了。
- 圖片是 PNG 與 SVG；SVG 裡的文字、內嵌點陣圖不畫；沒有 APNG 幀動畫。
- 沒有窗口出現 / 消失動畫、混合模式、圖層模糊、漸變字、變體覆蓋（用 `when` 分支與條件顏色代替）。
- 背景毛玻璃做不到：自繪點陣圖讀不到視窗後面的內容。
- 主題只能以目錄形式安裝，`extends` 只能繼承內建主題。
- Linux 上候選窗由 Fcitx5 / IBus 畫，主題不生效。
