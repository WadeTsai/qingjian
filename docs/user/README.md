# 使用者文件（官網「文件」頁的內容源）

這個目錄裡的 Markdown 是給**使用者**看的，官網（`qingjian-web`）構建時拉取整個目錄，按目錄結構渲染成文件頁。
`docs/` 下其他目錄是給開發者看的工程文件，不渲染。

這個 README 是維護約定，不渲染。

## 結構

```text
docs/user/
├── index.md              # 文件首頁
├── getting-started/      # 開始使用：安裝、第一次輸入、按鍵與快捷鍵（兩個平臺並排的總表）
├── input/                # 輸入：英文模式、拼寫糾錯、模糊音與輸入方案、快捷輸入
├── learning/             # 學習：譯詞與生詞、統計
├── cloud/                # 雲聯想（可選）
├── settings/             # 設定：偏好設定、詞庫
└── help/                 # 幫助：資料與日誌、解除安裝、反饋
```

- **一個資料夾是一個分組**，資料夾裡的 `index.md` 給分組標題與順序；資料夾裡的其他 `.md` 是分組下的頁面。
- **一個 `.md` 是一頁**。頂部 frontmatter：

  ```yaml
  ---
  title: 安裝            # 頁面標題，也是側欄裡的名字
  order: 1               # 同級排序，小的在前
  description: 一句話     # 搜尋引擎摘要與分享卡片用，40 字左右
  ---
  ```

- **檔名與資料夾名用 ASCII**（小寫、連字元），中文只寫在 `title` 裡；URL 就是路徑：`getting-started/install.md` → `/docs/getting-started/install`。
- **圖片放在同一資料夾**，相對路徑引用（`![候選視窗](candidate-window.png)`），構建時隨文件一起拷走。截圖用淺色外觀、系統預設字號、2x。
- 正文第一行不再寫 `# 標題`，標題來自 frontmatter；正文從 `##` 開始。
- 跨頁連結用相對路徑指向 `.md` 檔案（`[安裝](../getting-started/install.md)`），渲染時改寫成站內連結。

## 寫法

- 說明書體：陳述句，不用「一下 / 多半 / 照常 / 免得」這類口語助詞與副詞，不擬人、不調侃；條件用「若…則」，可行用「可」，必要用「需」；術語固定（候選視窗、拼音行、上屏、組合鍵）。
- 面向普通使用者，不出現實現詞（`config.toml`、marked text、Viterbi、panic、600 許可權這類）。要指路時說「偏好設定 → 進階」，不說檔名；
  資料檔案位置只在 [help/data-and-logs.md](help/data-and-logs.md) 一處列出。
- 按鍵按鍵帽寫：`Space`、`Enter`、`Backspace`、`Delete`、`Esc`、`Tab`、`←` `→` `↑` `↓`、`Home` / `End`、`PageUp` / `PageDown`；
  macOS 修飾鍵用符號（`⌥ + 數字`、`⌃⌥T`、`⇧ + Tab`），Windows 用單詞（`Ctrl + 數字`、`Ctrl + Alt + T`）；選單與頁名用「」：「系統設定 → 隱私與安全性」。
- 一個功能改了行為，同一個提交裡改這裡對應的頁；README 的安裝 / 基本用法 / 隱私一節與這裡重複的內容以這裡為準，README 只留概要。
- 事實來源是程式碼與 `docs/design/candidate-ui.md`（按鍵約定）、`docs/design/architecture.md`；寫之前對一遍，不憑印象。
