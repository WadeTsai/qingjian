---
title: 解除安裝
order: 2
description: 解除安裝青簡、連同學習資料一起刪除、只清除輸入日誌。
---

## macOS

開啟「終端」，執行：

```sh
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh
```

以上只刪除輸入法本身，學習資料與設定保留，重新安裝後仍可用。連同資料一起刪除：

```sh
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh --purge
```

此命令刪除「~/Library/Application Support/Qingjian/」中的全部內容，包括匯入的詞庫、學習到的詞、輸入日誌、配置與金鑰。

## Windows

在「設定 → 應用 → 安裝的應用」中找到「青簡」並解除安裝，或使用開始選單的「解除安裝青簡」。
正在執行的應用中的輸入法元件需等這些應用關閉後才能完全清除，解除安裝程式會提示是否重啟。

學習資料與設定保留在「%APPDATA%\Qingjian」，重新安裝後仍可用；連同資料一起刪除時，解除安裝後手動刪除該資料夾。

所有資料均為本機檔案，刪除後不可恢復，沒有云端副本。

## 只清除一部分

- 只清除輸入日誌：「偏好設定 → 進階 → 清空輸入日誌」。
- 刪除某個學到的詞：輸入拼音時按 `⇧ + 數字`，見 [譯詞與生詞](../learning/translation.md#刪除不需要的候選)。
- 移除匯入的詞庫：「偏好設定 → 詞庫 → 移除」只將檔案移到資料目錄的 `dicts/removed/`，需徹底刪除時自行清理該目錄。
