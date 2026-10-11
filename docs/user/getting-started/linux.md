---
title: Linux
order: 6
description: 在 Linux 上安裝青簡，使用 Fcitx5 預設候選面板。
---

青簡 Linux 提供預編譯包與原始碼安裝，使用 Fcitx5 預設候選面板。已驗證 Ubuntu 26.04 的 GNOME 桌面（Wayland）上 Fcitx5 5.1.19 的 GTK4、Qt6 應用與 Firefox；
其他桌面、其他發行版和舊版應用尚未完成驗證。當前支援本地候選、學習和本地整句模型重排，暫不提供雲聯想、設定視窗或自動啟動。

## 預編譯包

預編譯包在 Ubuntu 26.04（x86_64）上構建，外掛與系統的 Fcitx5 版本繫結；其他發行版請用下一節的原始碼安裝。
先裝好 Fcitx5 與 GTK / Qt 輸入支援：

```sh
sudo apt install python3 fcitx5 fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt6 fcitx5-config-qt
```

從[下載頁](https://qingjian.app/download)下載 `qingjian-<版本>-linux-x86_64.tar.gz`，解壓後在解出的目錄裡執行：

```sh
./install.sh

# 手動啟動，保持此終端執行
~/.local/bin/qingjian-linux-server
```

之後的新增輸入法、首次輸入與原始碼安裝相同，見下一節末尾。更新時下載新包再執行一次 `./install.sh`；解除安裝用包裡的 `./uninstall.sh`。

## 原始碼安裝和首次輸入

先安裝 Rust 1.96、CMake、C++20 編譯器、pkg-config、Fcitx5、其 GTK / Qt 輸入支援，以及 OpenSSL、Fcitx5 Core/Config/Utils 與 nlohmann-json 的開發包。
Debian / Ubuntu 上是：

```sh
sudo apt install cmake g++ pkg-config python3 libssl-dev \
  libfcitx5core-dev libfcitx5utils-dev libfcitx5config-dev nlohmann-json3-dev \
  fcitx5 fcitx5-frontend-gtk3 fcitx5-frontend-gtk4 fcitx5-frontend-qt6 fcitx5-config-qt
```

缺了哪一項，安裝指令碼開頭會直接列出來。
按發行版指引啟用 Fcitx5；X11 應用通常需要 `GTK_IM_MODULE=fcitx`、`QT_IM_MODULE=fcitx`、`XMODIFIERS=@im=fcitx`，環境變化後重新登入。

在原始碼目錄執行：

```sh
# 下載並校驗正式詞庫與本地整句模型
tools/release/data-fetch.sh
apps/linux/scripts/install.sh

# 手動啟動，保持此終端執行
~/.local/bin/qingjian-linux-server
```

也可用 `apps/linux/scripts/install.sh --sample --debug` 快速體驗少量樣例詞（例如「你好」），不下載正式詞庫。
預設安裝到 `~/.local`；`--prefix /絕對使用者目錄` 可更改安裝位置。請用相同使用者安裝、執行，不要使用 sudo。

重啟 Fcitx5，開啟 Fcitx5 配置工具，取消「僅顯示當前語言」，新增「青簡」。切換到青簡後輸入 `nihao`，空格選中「你好」。
單擊 `Shift` 切換中英；按鍵規則見 [按鍵與快捷鍵](keys.md#Linux（Fcitx5）)。關閉手動啟動的終端會結束青簡服務；下次登入後需再次啟動。

## 配置、隱私和資料

首次執行生成 `~/.config/qingjian/config.toml`，修改後重啟青簡服務。
`[general] preedit` 可設為 `both`（行內和候選視窗）、`inline`（只在行內）、`window`（只在候選視窗）；應用不支援行內顯示時使用候選視窗。
每頁候選數、翻頁鍵、學習、日誌和輔助語言使用同一配置檔案。`learning_language = "off"` 關閉中文候選的輔助語言釋義與生詞標記。系統面板外觀由 Fcitx5 設定控制。
當前預編譯包隨附含章·通變與含章·知微：優先用通變處理拼音整句與糾錯，沒有通變時回退到知微。自己的 `.qjm` 也可放入 `~/.local/share/qingjian/models/` 下對應的模型目錄。`[model] enabled = false` 可關閉本地模型，見 [本地整句模型](../input/local-model.md)。

`[general] shift_letter = "compose"` 讓 Shift 大寫字母參與中文組句，預設 `"passthrough"` 保持臨時英文輸入。
`scheme = "zhuyin"` 啟用注音（鍵盤排列 `zhuyin_layout = "dachen"` 標準或 `"eten"` 倚天）；雙拼下 `Shift + V` / `Shift + U` 可進入表示式 / 碼點輸入。
數字沒有對應候選時繼續輸入，英文直輸內容以空格結束時保留空格；英文候選開啟後可用數字、翻頁鍵、空格或 Tab 選詞。
具體規則見 [按鍵與快捷鍵](keys.md#Linux（Fcitx5）)。

Fcitx5 識別為敏感輸入時，可以組句但不會儲存輸入文本或學習；識別為密碼框或停用輸入法的輸入框時直接交還應用。
保護依賴應用和其輸入支援正確傳遞標記；本機已驗證 Qt6 的敏感標記，GTK4 的動態 PRIVATE 提示尚未傳遞為敏感輸入標記。
普通輸入的學習與日誌遵循配置。學習資料儲存在 `~/.local/share/qingjian`，執行日誌儲存在 `~/.local/state/qingjian/logs`。
設定了 `XDG_CONFIG_HOME`、`XDG_DATA_HOME`、`XDG_STATE_HOME` 時分別使用對應目錄下的 `qingjian`。

## 更新和解除安裝

結束手動啟動的青簡服務，重複執行安裝命令，然後重啟 Fcitx5 和青簡。安裝會檢查檔案歸屬；目標檔案被手動修改時會提示保留，請先備份處理。

```sh
apps/linux/scripts/uninstall.sh
# 自定義安裝位置時：
apps/linux/scripts/uninstall.sh --prefix /安裝時的絕對目錄
```

解除安裝後手動結束青簡服務並重啟 Fcitx5。解除安裝保留配置、個人詞庫、學習資料、已修改的安裝檔案和其他輸入法。
