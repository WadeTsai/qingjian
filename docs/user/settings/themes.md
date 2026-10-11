---
title: 主題
order: 3
description: 四個內建主題（青簡綠、系統藍、微信綠、櫻花）的選擇方法，以及自制主題的存放位置、寫法（顏色、漸變與圖片、陰影、字號與粗細、字型與斜體、文字描邊、伸出視窗的裝飾、過渡動畫、迴圈動畫）與生效方式。
---

主題決定候選視窗的顏色；macOS 與 Windows 上同一個主題顯示效果一致。主題只對青簡渲染器生效（「偏好設定 → 候選視窗 → 渲染引擎」選「青簡渲染器」）。

## 內建主題

| 主題 | 高亮的候選 |
|---|---|
| 青簡綠（預設） | 淺綠色底，文字顏色不變 |
| 系統藍 | 系統藍色底，白字 |
| 微信綠 | 微信綠色底，白字；雲端候選為藍色 |
| 櫻花 | 粉紫玻璃底、粉色高亮塊，宋體候選，詞性與釋義分兩行；左上角趴著的人物、櫻花掛件與緞帶。只有淺色 |

在「偏好設定 → 候選視窗 → 主題」中選擇。青簡綠、系統藍、微信綠都有淺色與深色兩套，按同一頁的「外觀」切換（跟隨系統 / 淺色 / 深色）；櫻花的底色是淺色圖案，始終用淺色。

## 自制主題

自制主題放在使用者資料目錄的 `themes` 資料夾中（使用者資料目錄的位置見[資料與日誌](../help/data-and-logs.md)），一個主題一個資料夾，資料夾裡放一個 `theme.json`：

```text
themes/
└── pink/
    └── theme.json
```

最簡單的寫法是以一個內建主題為底，只改幾種顏色。下面是一個粉色高亮的主題：

```json
{
  "extends": "qingjian",
  "schema": 1,
  "meta": { "id": "pink", "name": "粉色" },
  "variables": {
    "accent": { "light": "#ffb7d5", "dark": "#b0507a" },
    "hl_text": "#ffffff"
  }
}
```

- `theme.json` 裡可以寫註釋（`//` 到行尾，或 `/* … */`），物件和列表的最後一項後面多一個逗號也沒關係。
- `extends` 寫底子主題：`qingjian`（青簡綠）、`system-blue`（系統藍）、`wechat`（微信綠）或 `sakura`（櫻花）。沒寫的部分都沿用底子主題；底子主題的圖片不用拷進自己的資料夾。不寫 `extends` 就以青簡綠為底。
- `meta.id` 必須與資料夾名相同，且不能與內建主題重名，也不能是 `system`、`light`、`dark`；`meta.name` 是主題列表裡顯示的名字。
- 顏色寫作 `#rrggbb` 或帶不透明度的 `#rrggbbaa`；寫成 `{ "light": …, "dark": … }` 時淺色與深色外觀各用一個。

可改的顏色：

| 名字 | 用在 |
|---|---|
| `surface` | 視窗背景 |
| `accent` | 高亮候選的底色 |
| `text` | 候選詞 |
| `gloss` | 譯詞、拼音行 |
| `faint` | 序號、詞性、頁碼 |
| `fresh` | 生詞的譯詞 |
| `cloud` | 雲端候選與雲朵 |
| `hl_text`、`hl_gloss`、`hl_faint`、`hl_fresh`、`hl_cloud` | 高亮那一行裡對應的顏色 |

## 漸變與圖片

主題的視窗、高亮等部分除了純色，還可以用漸變或圖片填充，圖片放在主題資料夾裡，支援 PNG 與 SVG。例如把視窗背景換成一張可拉伸的邊框圖：

```json
"windows": {
  "vertical": {
    "fill": { "image": "images/frame.png", "slice": [16, 16, 16, 16], "scale": 2 }
  }
}
```

- `slice` 是圖片四邊（上、右、下、左）不拉伸的寬度，單位是圖片畫素：視窗變大時四角保持原樣，只拉伸中間；不寫則整張拉伸。
- `scale` 表示圖片是幾倍圖，按 2 倍尺寸繪製的圖片寫 2。
- SVG 圖片在任何縮放比例下都清晰，不用寫 `scale`。它的寬高就是顯示的大小（單位是點），`slice` 也按這個單位寫；按實際大小來畫，畫得過大會多佔記憶體。
- SVG 裡的文字不顯示，匯出前請轉成輪廓；SVG 裡嵌入的點陣圖和引用的外部檔案也不顯示，點陣圖請單獨用 PNG。
- 漸變寫作 `{ "linear": 90, "stops": ["#ffb7d5", "#ffffff"] }`（角度 90 為從左到右）。
- 背景是淺色圖片時，深色外觀下的文字會看不清，可在 `meta` 中加 `"appearance": "light"`，讓主題始終使用淺色。

## 陰影

視窗、高亮等部分可以加投影（`drop-shadow`）或內陰影（`inner-shadow`），寫在 `effects` 裡，可以疊幾層。例如把視窗陰影改淡、給高亮條加一層粉色投影：

```json
"windows": {
  "vertical": {
    "effects": [{ "type": "drop-shadow", "y": 4, "blur": 10, "color": "#00000030" }]
  }
}
```

- `x`、`y` 是偏移（向右、向下為正），`blur` 是模糊程度，`spread` 讓陰影向外擴（內陰影為向裡縮），單位都是點；不寫為 0。
- 陰影跟著形狀走：圓角視窗是圓角陰影，邊框圖片按圖片的透明邊投影，文字也可以加投影。
- 視窗陰影就是視窗本身的投影，寫 `"effects": []` 可以去掉。

## 字號與粗細

`text.styles` 裡有三種文字：`candidate`（候選詞）、`annotation`（譯詞與拼音行）、`index`（序號與頁碼）。每種可以改字號 `size`、行高 `line_height` 與粗細 `weight`：

```json
"text": {
  "styles": {
    "candidate": { "size": 18, "line_height": 22, "weight": "medium" }
  }
}
```

粗細寫 `light`（細）、`regular`（常規）、`medium`（中等）、`semibold`（半粗）、`bold`（粗），或 100–900 的數字。系統字型沒有某一檔粗細時，用最接近的一檔。

偏好設定裡填了「候選字號」「譯文字號」時，蓋過主題裡 `candidate`、`annotation` 的字號，行高按同樣比例縮放；`index`、`preedit` 與表格行高跟著候選字號一起縮放，同一行才對得齊。表格一行裡的各格按文字底線自動對齊，主題不用為不同字號調邊距。懸浮狀態條按候選字號的比例整體縮放，主題不用管。主題裡其它自己起名的樣式不受影響。

## 字型與斜體

每種文字可以用不同的字型，寫 `family`；斜體寫 `"style": "italic"`。寫在 `text` 下的 `family` 是沒單獨寫字型的文字都用的：

```json
"text": {
  "family": ["PingFang SC", "system"],
  "styles": {
    "candidate": { "size": 18, "line_height": 22, "family": ["Songti SC", "STSong", "system"] },
    "annotation": { "size": 12, "line_height": 15, "family": "Georgia", "style": "italic" }
  }
}
```

- `family` 寫一個字型名，或按順序寫幾個：用第一個裝了的。`system` 表示「偏好設定 → 候選視窗 → 字型」裡選的字型（沒選就是系統字型）；一個都沒裝時也用它。
- 字型名寫字族名，即系統字型列表裡顯示的英文名，如 `Songti SC`、`Microsoft YaHei`、`Georgia`。macOS 與 Windows 的字型不同，兩邊都要好看時把兩邊的名字都寫上。
- 字型裡沒有的字（比如英文字型遇到漢字）自動用系統字型補上。
- 字型沒有斜體時，把正體傾斜顯示；漢字的斜體也是這樣來的。

字型也可以隨主題帶：放進主題資料夾，在 `theme.json` 頂層列出來，`family` 寫字型本身的字族名：

```json
"fonts": [{ "file": "fonts/LXGWWenKai-Regular.ttf" }],
"text": { "styles": { "candidate": { "size": 18, "line_height": 22, "family": "LXGW WenKai" } } }
```

支援 TTF、OTF、TTC，單個檔案不超過 32 MB。只帶許可允許再分發的字型（如 OFL 許可的霞鶩文楷），系統自帶的字型只寫名字、不要拷進主題。

## 文字描邊

候選詞、譯詞、拼音行的文字可以加描邊，寫在對應的文字上：`"stroke": { "width": 1.5, "color": "#c2386b" }`，`width` 單位是點。
描邊向字的外側加寬，不改變文字佔的位置。顏色可以按是否高亮區分，例如只給高亮的候選描邊：

```json
"stroke": { "width": 1.5, "color": { "if": "highlighted", "then": "#c2386b", "else": "#00000000" } }
```

文字陰影與發光用陰影的寫法：在文字上加 `effects`，例如 `[{ "type": "drop-shadow", "blur": 4, "color": "#e85a8c" }]` 就是一圈粉色光暈。
彩色 emoji 不描邊。

## 伸出視窗的裝飾

裝飾圖可以探出候選視窗的邊緣（比如趴在視窗角上的小角色）。在視窗的 `children` 里加一個絕對定位的框，`inset`（上、右、下、左到視窗邊的距離）寫負數就伸到視窗外面：

```json
{ "type": "frame", "position": "absolute", "inset": [-24, -24, "auto", "auto"], "width": 48, "height": 48,
  "fill": { "image": "images/cat.png", "scale": 2 } }
```

上例是右上角探出 24 點。候選視窗仍按視窗本身貼著游標擺放，裝飾不影響位置，貼近螢幕邊緣時裝飾可能露在螢幕外。裝飾不擋滑鼠。

## 詞性與釋義分開排

譯文預設排成一行（`n. pure letters · …`）。想讓詞性和釋義各佔一行，在候選裡寫兩個譯文節點，用 `tones` 指定每個只畫哪幾種內容，`senses` 指定只畫前幾個義項：

```json
{ "type": "annotation", "bind": "annotation", "font": "annotation", "tones": ["pos"], "senses": 1, "height": 15,
  "gloss": "@gloss", "fresh": "@fresh", "faint": "@faint" },
{ "type": "annotation", "bind": "annotation", "font": "annotation", "tones": ["gloss", "fresh"], "senses": 1,
  "gloss": "@gloss", "fresh": "@fresh", "faint": "@faint" }
```

- `tones` 可寫：`pos`（詞性）、`gloss`（釋義）、`fresh`（生詞的釋義）、`separator`（義項之間的 ` · `）、`faint`（日文的假名注音）、`code`（輔碼）。不寫全畫。
- 有的候選沒有詞性（比如 emoji），給詞性那一行寫上 `height`，空著也佔住位置，幾列才對得齊。
- 詞性和分隔符的顏色可以單獨寫 `pos`、`separator`，不寫用 `faint`。

## 顯示輸入狀態

候選窗口裡可以顯示當前是中文還是英文、簡體還是繁體、標點全形還是半形，以及輸入方案名。顯示什麼字由主題自己寫：
用 `when` 讓兩個文字只出現一個，方案名用 `"bind": "mode.scheme"`：

```json
{ "type": "text", "text": "中", "when": "!mode.english", "font": "index", "color": "@gloss" },
{ "type": "text", "text": "英", "when": "mode.english", "font": "index", "color": "@gloss" },
{ "type": "text", "text": "繁", "when": "mode.traditional", "font": "index", "color": "@gloss" },
{ "type": "text", "bind": "mode.scheme", "font": "index", "color": "@faint" }
```

- `mode.english`：英文模式；`mode.traditional`：繁體輸出；`mode.full_width`：標點是全形。前面加 `!` 表示「不是」。
- `mode.scheme` 是方案名，如「全拼」「小鶴雙拼」「五筆（86） + 全拼」；寫在 `when` 裡表示「有方案名」。
- 這幾個節點放在視窗的 `children` 裡；`children` 要整組寫出來（只寫新加的會把原來的內容整個換掉），可以從內建主題的 `theme.json` 複製一份再改。

## 懸浮狀態條（Windows）

Windows 螢幕右下角的懸浮狀態條也可以按主題畫，寫法與候選視窗相同：在 `status` 下寫 `root`，格子用 `repeat` 繫結 `cells` 一格一格展開（現在是「中 / 英」「標點」「設定齒輪」三格，點哪格就是切換哪一項）：

```json
"components": {
  "status-cell": { "type": "frame", "padding": [4, 8], "children": [
    { "type": "text", "bind": "text", "font": "candidate",
      "color": { "if": "emphasized", "then": "@cloud", "else": "@gloss" } },
    { "type": "icon", "icon": "gear", "when": "gear", "size": 15, "color": "@gloss" }
  ] }
},
"status": {
  "root": { "type": "frame", "fill": "@surface", "radius": 8, "children": [
    { "type": "frame", "width": 24, "height": 24, "fill": { "image": "images/sakura.svg" } },
    { "type": "repeat", "bind": "cells", "component": "status-cell" }
  ] }
}
```

- 每一格里可用：`text`（這一格的文字）、`emphasized`（當前生效的模式或全形標點）、`gear`（設定齒輪那一格）、`first` / `last`。
- 整條狀態條也能用「顯示輸入狀態」裡的 `mode.english` 等，自己決定畫什麼。
- 格子之間可以夾裝飾，點到裝飾算前一格。
- 不寫 `root` 時狀態條按原來的樣子畫，只用 `status` 裡的顏色與尺寸。macOS 沒有懸浮狀態條。

## 過渡動畫

高亮換到另一個候選時，高亮條可以滑過去，而不是直接跳（三個內建主題都是這樣）。給高亮條起個名字（`id`）並加上 `transition`：

```json
{ "type": "frame", "id": "highlight", "transition": { "duration": 120, "easing": "ease-out" },
  "when": "highlighted", "fill": "@accent", "radius": 4 }
```

- 前後兩次顯示中同名的部分會從舊的位置、大小、不透明度過渡到新的；顏色不過渡。
- `duration` 是時長（毫秒）；`easing` 是快慢曲線：`linear`（勻速）、`ease`、`ease-in`（先慢後快）、`ease-out`（先快後慢）、`ease-in-out`，
  或寫四個數 `[x1, y1, x2, y2]`（同網頁的 `cubic-bezier`）。
- 偏好設定裡關了「過渡動畫」，或系統開了「減少動態效果」（macOS：系統設定 → 輔助功能 → 顯示；Windows：設定 → 輔助功能 → 視覺效果 → 動畫效果關閉）時不播動畫。

## 迴圈動畫

裝飾可以一直輕輕地動，比如飄動的花瓣、呼吸的光暈。在節點上加 `animation`，用關鍵幀寫它在一輪裡怎麼動：

```json
"animation": {
  "duration": 2400, "easing": "ease-in-out",
  "keyframes": [
    { "at": 0, "y": 0, "rotate": 0 },
    { "at": 0.5, "y": 3, "rotate": 20 },
    { "at": 1, "y": 0, "rotate": 0 }
  ]
}
```

- `duration` 是一輪的時長（毫秒），`at` 是關鍵幀在一輪裡的位置（0 到 1）。
- 可以動的有：`x`、`y`（平移，點）、`rotate`（旋轉角度，繞自身中心）、`scale`（縮放）、`opacity`（不透明度）。關鍵幀裡沒寫的就保持不動。
- 動的是這個部分連同它裡面的所有內容，不會擠動別的部分。
- 預設一直迴圈；寫 `"loop": false` 只播一次，停在最後的樣子。
- 候選窗口出現時從頭播，打字過程中不重播；關了「過渡動畫」或開著「減少動態效果」時不動。

## 生效方式

- 新放入的主題：重新開啟偏好設定後出現在「主題」列表中。
- 修改正在使用的主題：儲存 `theme.json` 後，在輸入狀態下約一秒內生效，可以邊改邊看。
- 主題檔案寫錯時，該主題不出現在列表中；正在使用的主題讀不進來時，候選視窗改用青簡綠。原因記錄在執行日誌中（見[資料與日誌](../help/data-and-logs.md)）。
