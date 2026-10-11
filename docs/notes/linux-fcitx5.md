# Linux Fcitx5 預設面板

第一階段保留 Rust Server、薄外掛和使用者目錄安裝。輸入模式、組句、選詞、翻頁、隱私和失焦提交均在 Server；
外掛負責 Fcitx 事件與預設預編輯 / 候選 API。長期自繪方向不變，後續由 Server 渲染並管理 X11 視窗，或向 GNOME 擴充套件傳送點陣圖；本階段不帶相關原始碼、補丁、渲染 FFI、服務管理或 Debian 包。

## 構建與測試

需要 Rust 1.96、CMake 3.16、C++20 編譯器、pkg-config，以及 OpenSSL（`libssl-dev`，雲服務依賴的 TLS 繫結要它）、Fcitx5 Core/Config/Utils 和 nlohmann-json 的開發包；`install.sh` 開頭會檢查並列出缺的。
構建最低 API 為 Fcitx5 5.1.8，CI 使用 Ubuntu 26.04 的 Fcitx5 5.1.19；Ubuntu 24.04 自帶的 5.1.7 不滿足要求。
桌面執行已驗證的範圍見下文，構建版本門檻不代表全部桌面已驗收。

```sh
cargo test -p qingjian-linux-server --locked
cargo fmt --all --check
cargo clippy --workspace --exclude qingjian-macos --all-targets --locked -- -D warnings
cargo test --workspace --exclude qingjian-macos --locked
cargo build -p qingjian-linux-server --locked
cmake -S apps/linux/fcitx5 -B target/fcitx5-stage1 -DCMAKE_BUILD_TYPE=Debug -DBUILD_TESTING=ON
cmake --build target/fcitx5-stage1 --parallel 2
env -u DBUS_SESSION_BUS_ADDRESS ctest --test-dir target/fcitx5-stage1 --output-on-failure
python3 apps/linux/tests/installation.py
```

CTest 的 `real-server` 實際執行 Rust Server 並通過 InputContext 輸入與點選，預設二進位制位置 `target/debug/qingjian-linux-server`。
自定義 Cargo target 時傳 `-DQINGJIAN_SERVER_EXECUTABLE=/絕對路徑/qingjian-linux-server`。
測試例項關閉所有 Fcitx addon，不依賴桌面匯流排；外掛初始化本身也不連線 D-Bus。真實桌面測試才需要 `fcitx5-modules`、GTK / Qt 輸入模組及 `dbus-run-session`。

## 協議與狀態

同用戶 Unix socket，長度字首為 4 位元組小端，單訊息上限 16 MiB；每次外掛收發共用 200 ms 截止時間。
外掛只持有一條連線和一個 I/O watcher，每個 InputContext 分配獨立、遞增的 session ID。
每個會話先 `OpenSession` 校驗共享協議版本（`PROTOCOL_VERSION`），再協商 `LinuxHello` v3，攜帶 session/generation/context，返回會話編號和既有預編輯設定。
首次能力確認前不接受輸入；握手與能力狀態逐會話儲存。同一連線支援超過 64 個上下文，64 條連線的資源保護不限制單連線會話數。
Linux 的首個回包仍是含 `linux_ui` 的 `Update`，不額外發送 Windows DLL 使用的 `SessionOpened`；舊版外掛的協議 5 連線會被拒絕，升級時需同時更新 Server 與外掛。
舊版本或損壞響應關閉連線、清空顯示並放行當前輸入；重連建立全新輸入狀態，舊提交不會重放。
顯示 API 同步觸發能力變化、失焦或 Reset 時，外掛在最終上屏前撤銷舊生命週期響應，但保持已接受按鍵的 Consumed 結果；純顯示失敗仍交付有效提交。

同一上下文停用後保留會話和中英模式；Reset 只丟棄輸入。Password / Disable 關閉目標會話，恢復普通輸入後分配新編號。
銷燬上下文時撤銷安全引用並排隊 CloseSession，由事件迴圈延遲傳送，不阻塞析構，也不關閉其他會話。
共享連線故障先讓全部會話失效，再清理各自顯示；清理回撥期間不重連。

LinuxEvent 轉發能力、按下 / 釋放、焦點、停用原因、客戶端預編輯事實、帶幀身份的候選點選和翻頁。
空能力是普通輸入；Sensitive 可組句但不學習、不記輸入文本；Password / Disable 優先停用並丟棄輸入。
能力先由 Server 確認，再清理可能同步重入的預編輯；Reset / 焦點變化不會丟掉能力通知，清理中重入的新按鍵已受新隱私狀態約束。
能力變化清理組句、暫存透傳、學習鏈與補全；能力即將改變時的 deactivate 原因也觸發清理，即使框架此刻仍返回舊能力。
FocusOut 的行內預編輯由框架或宣告 ClientUnfocusCommit 的客戶端提交，Server 不重複返回它；僅視窗預編輯才返回原樣組句。

每次候選響應繫結 generation/context/revision，revision 在整個 Server 內遞增；過期或跨會話點選返回 Ignored，不改變當前會話的展示狀態。
預設面板只展示第一條釋義，完成面板更新後報告當前頁對應的 `(候選槽位, 0)`。
隱藏、失焦、私密、無候選與過期回報不產生有效展示記錄；Server 只憑已生成幀不記展示。

組句期間外掛每 80 ms 發一次 `Poll`（與 Windows DLL 的輪詢間隔相同），Server 先 tick 再回當前幀。
幀與該會話上次發出的相同時沿用展示身份（revision 不變），外掛不重畫也不重複回報；本地整句模型換了順序才推進 revision，外掛按版本號推進重畫並回報。
幀為空、Reset、失焦、停用、斷線時外掛停表；協議版本不變。

## 本地整句模型

Server 啟動時按 `[model] enabled`（預設開）在後臺執行緒載入模型並預熱，`find_model` 優先找使用者目錄 `~/.local/share/qingjian/models/hanzhang-tongbian/` 與隨包的 `resources/data/models/hanzhang-tongbian/`，然後回退知微；舊使用者目錄仍可讀取。Linux 沒有配置熱載入，改開關要重啟服務。
節拍與 Windows Server 的 `dispatch/rescore` 相同：緩衝變化後起 80 ms 防抖，到點把整句路徑送去後臺打分，每 20 ms 收一次，最長等 2 s；
結果到了只在使用者還看著第一頁、沒動過高亮時重建候選佈局，下一次 `Poll` 回的幀就是新順序。主迴圈按 `Router::next_tick` 的絕對到點時間等訊息，空閒時一秒一次落盤學習。
模型在後臺接上時使用者正在組句，Server 補查一次攢下整句路徑再起防抖，這一輪不錯過重排。前文用本會話最近上屏的字，首版不讀應用游標前文。

## 路徑和排錯

使用者安裝與手動啟動見 [Linux 使用者說明](../user/getting-started/linux.md)。安裝只登記實際絕對外掛庫路徑，不修改系統 Fcitx5 搜尋規則。
`QINGJIAN_SOCKET` 可指定絕對 socket 路徑；預設為 `$XDG_RUNTIME_DIR/qingjian.sock`，無 runtime 時用 `/tmp/qingjian-<uid>/qingjian.sock`。
父目錄須歸當前使用者且不可被其他使用者寫入；socket 許可權為 0600。`QINGJIAN_RESOURCES` 覆蓋資源根，`QINGJIAN_DICT` 可指定測試詞庫。
預設資料從執行檔旁 `../share/qingjian/resources` 找，支援原始碼開發目錄回退。

未出候選時先確認手動 Server 在執行，再檢視 `$XDG_STATE_HOME/qingjian/logs/server.*.log`（預設 `~/.local/state`）。
輸入日誌由 `[general] input_log` 控制；使用者配置和學習檔案始終儲存在 XDG 使用者目錄，解除安裝不刪除。

## 驗證環境

- Ubuntu 26.04，未打補丁的 Fcitx5 5.1.19-1 / Core7 / modules，GTK4 4.22.4、Qt6 6.10.2。
- 隔離 X11（Xvfb）、私有 D-Bus：GTK4、Qt6。
- 真實桌面（2026-09-20）：GNOME 50 原生 Wayland，文本編輯器（GTK4）、FeatherPad（Qt6）、Firefox（snap）。Ubuntu 26.04 的 GNOME 已沒有 X11 登入項。
  預設面板的外觀與位置由 Fcitx5 決定：GTK4 應用裡是輸入模組自己畫的黑白麵板，Firefox 裡是 Fcitx5 的橙色面板且與游標同一行，Fcitx5 自帶拼音表現相同。
- 真實桌面（2026-09-22，本地整句模型）：CachyOS，niri（原生 Wayland），Fcitx5 5.1.22，Microsoft Edge（Chromium，`--enable-wayland-ime`）與 Konsole（Qt6）：停頓後整句候選按模型換序。
- 未驗證：KDE 與其他桌面、其他發行版、GTK3 / Qt5 應用、真實 X11 桌面會話。
