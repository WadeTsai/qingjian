# 安全政策 / Security Policy

## 支援的版本

青簡還在測試階段，只修最新版本：

| 版本 | 支援 |
|---|---|
| 最新的 0.1.x | ✅ |
| 更早 | ❌，請升級 |

## 報告漏洞

**不要在公開 issue 裡寫漏洞細節。** 請用 GitHub 的私密漏洞報告：倉庫頁面「Security」→「Report a vulnerability」。

收到後 3 天內回覆，確認後在下一個版本修復並在更新日誌裡致謝（除非你不願意署名）；不算漏洞的會說明原因。

輸入法能看到你敲的每一個鍵，所以我們特別在意這些：

- 敲的內容外洩：日誌、診斷資訊、雲端請求裡出現不該出現的輸入；密碼框（Secure Input）裡仍在組句或發請求。
- 金鑰：`.env` / 配置裡的 API 金鑰被寫進日誌、診斷資訊或別的檔案。
- 安裝包完整性：Release 上的 pkg 與 `SHA256SUMS`、`releases.json` 裡的雜湊對不上，或簽名有問題。
- 解析崩潰：惡意構造的詞庫檔案（TSV、Rime yaml、`.qj`）、配置檔案或雲端返回讓輸入法崩潰或越界。

不在範圍內：需要本機管理員許可權或物理接觸才能利用的問題；你自己填寫的 AI 服務商那一側的問題。

---

**English.** Qingjian is in beta; only the latest 0.1.x release receives fixes. Please report vulnerabilities privately via
GitHub's "Report a vulnerability" (Security tab), not in public issues. Expect a reply within 3 days; confirmed issues are fixed
in the next release and credited in the changelog. An input method sees every keystroke, so we care most about keystroke leakage
(logs, diagnostics, cloud requests, Secure Input), API key exposure, installer integrity (pkg vs. `SHA256SUMS`), and crashes from
malformed dictionaries, config or cloud responses.
