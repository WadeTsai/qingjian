# 含章模型名稱與檢查點

「含章」是青簡本地神經模型系列。下面的檢查點編號來自現有模型配置，用於追溯權重；兩款模型尚未指定正式發行版本。應用版本和資料 Release 標籤也不充當模型版本。

| 模型 | 英文名 | 標識 | 用途 | 當前檢查點 |
| --- | --- | --- | --- | --- |
| 含章·通變 | Hanzhang Tongbian — neural pinyin decoding and correction | `hanzhang-tongbian` | 拼音到漢字解碼、糾錯、整句候選重排與詞圖未覆蓋時的生成 | `small-15000` |
| 含章·知微 | Hanzhang Zhiwei — context-aware candidate rescoring | `hanzhang-zhiwei` | 依據游標前文給整句候選重排；沒有通變時作為回退 | `small-155478` |

`tools/release/pack-model.sh` 把正式名稱寫進 `.qjm` 的 `name` 後設資料，`version` 暫記型號 `hanzhang-tongbian-small` / `hanzhang-zhiwei-small`，不附檢查點編號。檔名與型號相同，分別放在 `models/hanzhang-tongbian/` 與 `models/hanzhang-zhiwei/`；檢查點仍記錄在上表和原始配置中，資料 Release 用獨立的 `data-vN` 標籤和 SHA-256 鎖定具體位元組。

改名稱或後設資料時，兩份模型都要重新打包並重算雜湊；不能只改指令碼而沿用舊 `.qjm`。正式釋出時將兩份模型和詞庫裝進同一個執行時壓縮包，並更新 `tools/release/data.lock`。
