# qingjian-macos

macOS 輸入法殼（InputMethodKit）。按鍵進 Core 的 `Engine`，候選畫在自繪 NSPanel；原始碼按 `app / host / imk / candidates / menubar / preferences` 分目錄，
架構見 `docs/design/architecture.md`「macOS：IMK」。

## 開發安裝

```bash
apps/macos/scripts/bundle.sh --install
```

打包到 `target/Qingjian.app` 並裝到 `~/Library/Input Methods/`，殺掉舊程序；切換一次輸入法就會拉起新的。
首次要在「系統設定 → 鍵盤 → 輸入法 → 編輯 → +」的簡體中文下新增「青簡」，列表裡沒有就登出再登入。
日誌在 `~/Library/Logs/Qingjian/`，使用者資料與 `config.toml` 在 `~/Library/Application Support/Qingjian/`。

## 分發 pkg

```bash
apps/macos/scripts/bundle.sh --pkg        # target/pkg/qingjian-<版本>-macos-<arch>.pkg
```

pkg 裝到 `/Library/Input Methods/`（需要管理員密碼），裝完 postinstall 以登入使用者身份跑 `qingjian-macos --register`（註冊、啟用並切成當前輸入源），
輸入源自動註冊並啟用，不用手動新增。版本號取 workspace `Cargo.toml`，構建號是提交數。
機器上同時有 `~/Library` 的開發副本時兩份會互相頂，先跑 `scripts/uninstall.sh`。

簽名與公證靠環境變數，沒設就 ad-hoc 籤 `.app`、pkg 不籤（測試者首次開啟要在「系統設定 → 隱私與安全性」裡點「仍要開啟」）：

| 變數 | 作用 |
|---|---|
| `QINGJIAN_SIGN_IDENTITY` | `Developer ID Application: …`，給 `.app` 簽名並開 hardened runtime |
| `QINGJIAN_INSTALLER_IDENTITY` | `Developer ID Installer: …`，給 pkg 簽名 |
| `QINGJIAN_NOTARY_PROFILE` | `xcrun notarytool store-credentials` 存的 profile 名，設了就公證並 staple |

二進位制只有本機架構（`hostArchitectures` 按 `uname -m` 寫進 Distribution），Intel 機器要另打一份。

## 解除安裝

```bash
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh          # 保留學習資料與配置
/Library/Input\ Methods/Qingjian.app/Contents/Resources/uninstall.sh --purge  # 連資料、配置、日誌一起刪
```

倉庫裡的 `apps/macos/scripts/uninstall.sh` 是同一個檔案，兩處安裝位置都會清。
