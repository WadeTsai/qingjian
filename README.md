<p align="center">
  <img src="assets/icon/qingjian-mark.svg" alt="青簡竹簡圖示" height="108">
</p>

<h1 align="center">青簡 Qingjian</h1>

<p align="center"><strong>好好輸入，順便多認識一個詞。</strong></p>

<p align="center">
  <a href="https://qingjian.app/download"><img src="https://img.shields.io/github/v/release/qingjian-team/qingjian?label=stable" alt="stable release"></a>
  <a href="https://github.com/qingjian-team/qingjian/stargazers"><img src="https://img.shields.io/github/stars/qingjian-team/qingjian?style=flat&amp;label=Stars" alt="GitHub Stars"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="License: GPL-3.0-or-later"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/macOS-13%2B-blue" alt="macOS 13+"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/Windows-10%2F11-blue" alt="Windows 10/11"></a>
  <a href="https://qingjian.app/docs/getting-started/linux"><img src="https://img.shields.io/badge/Linux-Fcitx5%20manual-lightgrey" alt="Linux Fcitx5，手動啟動"></a>
</p>

青簡是一款輸入法。你可以像平常一樣打字：輸入拼音、選擇候選、寫完整句；候選旁的一條譯詞，讓語言學習自然發生在日常輸入裡。譯詞始終只是輔助資訊，不會蓋過你要輸入的文字。

https://github.com/user-attachments/assets/d145fde9-a641-4543-8b15-dd7a2685de3d

影片演示了青簡在 macOS 上的整句輸入、候選重排和候選譯詞。

## 下載與開始使用

- **macOS、Windows**：[下載青簡](https://qingjian.app/download)；安裝步驟見[使用文件](https://qingjian.app/docs/getting-started/install)。
- **Linux**：已有 Fcitx5 版本，使用系統預設候選面板；目前需要手動啟動後臺服務，詳見 [Linux 安裝說明](https://qingjian.app/docs/getting-started/linux)。

macOS 與 Windows 版本仍處於測試階段。安裝後先選中青簡，在「偏好設定 / 設定 → 通用」選擇想學習的語言，就可以開始輸入。第一次使用可從[第一次輸入](https://qingjian.app/docs/getting-started/first-input)讀起。

## 輸入時，你會看到什麼

```text
1  開發        development
2  程式設計        programming
3  架構        architecture
```

候選旁一次只顯示一種學習語言的譯詞。目前可以選擇英語、日語或西班牙語，也可以關閉譯詞顯示。青簡還支援整句輸入、簡拼、拼寫糾錯、雙拼、五筆等輸入方式；本地整句模型會在停頓後調整句子候選。

「統計」頁還會顯示今天、最近 7 天和累計的輸入量，以及學習語言的詞彙記錄。詳細用法見[輸入功能](https://qingjian.app/docs/input)、[譯詞與生詞](https://qingjian.app/docs/learning/translation)、[本地統計](https://qingjian.app/docs/learning/statistics)和[按鍵與快捷鍵](https://qingjian.app/docs/getting-started/keys)。

## 資料與隱私

拼音轉換、詞庫查詢、本地模型和輸入習慣學習在你的裝置上完成。輸入量與詞彙統計也只儲存在本機，不會上傳；青簡不需要賬號。輸入日誌與統計分開儲存，日誌可在設定中關閉或清空，不影響統計。檢查更新會向官網請求版本列表，可在設定中關閉。

可選的**雲聯想預設關閉**。開啟後，青簡會把當前輸入和附近的文字直接傳送給你自行填寫的 AI 服務商，以獲取候選或整句補全；請求不經過青簡的伺服器。傳送範圍與本機儲存的資料，見[資料與日誌](https://qingjian.app/docs/help/data-and-logs)。

## 文件、反饋與參與開發

- [使用文件](https://qingjian.app/docs)：安裝、設定、輸入、解除安裝與常見問題。
- [反饋問題或建議](https://github.com/qingjian-team/qingjian/issues/new/choose)；也可以加入 [QQ 內測交流群](https://qm.qq.com/q/jBvn2gGTxm)。
- 想參與開發？從[開發文件](docs/)和[開發約定](docs/contributing.md)開始。

青簡在[官方渠道](https://qingjian.app/download)免費提供。程式碼採用 [GPL-3.0-or-later](LICENSE) 許可；專案名稱與 logo 不包含在程式碼授權中。隨包資料有各自的來源與許可，見[資料來源清單](docs/design/landscape.md)。
