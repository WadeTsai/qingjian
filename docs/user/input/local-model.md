---
title: 本地整句模型
order: 5
description: 隨包的小模型在本機給整句候選重新排序：怎麼生效、什麼時候不生效、怎麼關。
---

青簡的本地模型系列叫「含章」。含章·通變（Hanzhang Tongbian）負責拼音解碼與糾錯；含章·知微（Hanzhang Zhiwei）負責根據上下文重排候選。優先使用通變，裝置上沒有它時使用知微。全程離線，不聯網、不傳送任何內容。

## 怎麼生效

敲完一段拼音停一下（不到十分之一秒），整句候選就換成模型認為更通順的那句；沒停就按詞庫統計出的先顯示。
通變根據這段拼音選句，不讀取游標前文。回退到知微時，模型會參考游標前面的幾十個字；應用不讓讀上下文時（部分終端、Electron 應用）只看這次會話裡輸入過的字，Linux 目前也只看本次會話的字。

模型會調整整句候選；詞庫拼不出完整句子時，還可能補出整句候選。詞候選、你選過的詞都不受它影響；你的習慣仍然優先。

macOS 14 等缺少所需 Metal 功能的系統會自動用 CPU 執行本地模型，載入和重排可能更慢。

## 什麼時候不生效

- 已經翻到後面的頁或用方向鍵移過高亮時，模型的結果不再換掉正在看的這頁。
- 密碼框裡不組句；應用宣告為私密的輸入框裡不讀上下文。
- 輸入法剛啟動的幾秒鐘模型還在載入，這段時間按詞庫統計出句。

## 關掉

「偏好設定 → 雲服務」（Windows：「設定 → 雲服務」）關閉「本地整句模型」，或配置檔案 `[model]` 裡 `enabled = false`（Linux 只有這一種，改完重啟青簡服務）。關閉後只用詞庫統計，與雲聯想互不影響。

## 自己的模型

把 `hanzhang-tongbian-small.qjm` 放進使用者資料目錄的 `models/hanzhang-tongbian/`，重啟輸入法後優先使用。含章·知微使用 `hanzhang-zhiwei-small.qjm`，放在 `models/hanzhang-zhiwei/`，僅在沒有通變時使用。舊使用者目錄 `model-p2c/` 與 `model/` 仍可讀取；同一模型的新目錄優先。直接匯出的 `model.safetensors`、`config.json`、`vocab.json` 三個檔案也能識別；同目錄裡兩種格式都有時優先用 `.qjm`：

- **macOS**：`~/Library/Application Support/Qingjian/models/hanzhang-tongbian/`
- **Windows**：`%APPDATA%\Qingjian\models\hanzhang-tongbian\`
- **Linux**：`~/.local/share/qingjian/models/hanzhang-tongbian/`
