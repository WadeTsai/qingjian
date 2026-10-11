//! 上屏：高亮候選、指定候選、拼音原樣。

use super::*;

impl QingjianInputController {
    pub(super) fn commit_highlighted(&self, client: TextClient<'_>) -> bool {
        let index = host::with(|h| h.session.highlighted).unwrap_or(0);
        self.commit_index(index, client)
    }

    /// 上屏第 `index` 個候選；沒有候選時上屏拼音本身。上屏後剩餘拼音繼續組句。
    pub(super) fn commit_index(&self, index: usize, client: TextClient<'_>) -> bool {
        let candidate = host::with(|h| h.session.candidate(index)).flatten();
        let Some(candidate) = candidate else {
            if host::with(|h| index < h.session.layout.len()).unwrap_or(false) {
                return true;
            }
            return self.commit_raw(client);
        };
        let Some(text) = host::with(|h| h.engine.commit(&candidate)) else {
            return false;
        };
        tracing::debug!(%text, "commit");
        client.insert_text(&text);
        self.refresh(client);
        true
    }

    /// 把拼音原樣上屏並清空。緩衝區為空時返回 false。
    pub(super) fn commit_raw(&self, client: TextClient<'_>) -> bool {
        let Some(raw) = host::with(|h| h.engine.take_raw()) else {
            return false;
        };
        if raw.is_empty() {
            return false;
        }
        tracing::debug!(%raw, "commit raw");
        client.insert_text(&raw);
        self.refresh(client);
        true
    }
}
