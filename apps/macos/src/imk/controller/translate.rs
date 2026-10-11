//! 翻譯選中文字、譯詞鍵與刪候選鍵。

use super::*;

impl QingjianInputController {
    /// Option+數字：上屏當前頁第幾個候選的譯文（學習和拼音消耗與選那個候選一樣）。
    /// 不在組句中時不管；候選沒有譯文就吞掉按鍵不動，免得 ¡™£ 進應用。
    /// 翻譯應用裡選中的文字：雲服務關著、密碼框、沒有選區都不動（鍵交回應用）。
    pub(super) fn translate_selection(&self, client: TextClient<'_>) -> bool {
        if !host::with(|h| h.engine.prediction_enabled()).unwrap_or(false) {
            tracing::info!("云服务没开，翻译快捷键不生效");
            return false;
        }
        if secure_input::enabled() {
            tracing::debug!("Secure Input 中，不翻译");
            return false;
        }
        let Some((text, range)) = client.selected_text(MAX_TRANSLATE_CHARS) else {
            // 分不清是沒選還是應用不給讀（不少 Electron 應用不支援），兩種情況都提示一下，鍵吞掉
            tracing::debug!("没有选中的文字，或应用不支持读选区");
            let anchor = client.caret_rect();
            host::with(|h| {
                h.show_notice(
                    "没有选中的文字，或这个应用不支持读取选区（最多 500 字）",
                    anchor,
                )
            });
            return true;
        };
        // 游標位置先在借用之外取好：取的過程會等應用回話，期間別的 IMK 回撥可能重入
        let anchor = client.caret_rect();
        let sent = host::with(|h| {
            h.anchor = anchor;
            h.engine.request_translation(&text).is_some()
        })
        .unwrap_or(false);
        if !sent {
            return false;
        }
        tracing::debug!(chars = text.chars().count(), "翻译选中文字");
        host::with(|h| h.begin_translation(range));
        true
    }

    /// 翻譯視窗開著時的按鍵：回車 / 空格 / 1 用譯文替換選區，Esc 放棄；其他鍵放棄並交回應用。
    pub(super) fn handle_translation_review(&self, key: u16, client: TextClient<'_>) -> bool {
        let job = host::with(|h| h.translation.clone()).flatten();
        let Some(job) = job else {
            return false;
        };
        match key {
            // 回車 / 小鍵盤迴車 / 空格 / 1：接受（譯文還沒到時先等）
            36 | 76 | 49 | 18 => {
                if let Some(result) = job.result {
                    tracing::debug!("接受译文");
                    client.replace_range(&result, job.range);
                    host::with(|h| h.end_translation());
                }
                true
            }
            // Esc：放棄
            53 => {
                host::with(|h| h.end_translation());
                true
            }
            _ => {
                host::with(|h| h.end_translation());
                false
            }
        }
    }

    pub(super) fn handle_translation_key(
        &self,
        digit: usize,
        sense: usize,
        client: TextClient<'_>,
    ) -> bool {
        let index = host::with(|h| h.session.index_on_page(digit - 1)).flatten();
        self.commit_translation_at(index, sense, client)
    }

    /// 上屏排布里第 `index` 格候選的第 `sense` 條譯詞；數字鍵與點選共用。不在組句中返回 `false`。
    pub(super) fn commit_translation_at(
        &self,
        index: Option<usize>,
        sense: usize,
        client: TextClient<'_>,
    ) -> bool {
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return false;
        }
        let candidate =
            index.and_then(|index| host::with(|h| h.session.candidate(index)).flatten());
        let text = candidate
            .and_then(|c| host::with(|h| h.engine.commit_translation(&c, sense)).flatten());
        match text {
            Some(text) => {
                tracing::debug!(%text, "commit translation");
                client.insert_text(&text);
                self.refresh(client);
            }
            None => tracing::debug!(?index, sense, "这个候选没有这条译文"),
        }
        true
    }

    /// 修飾鍵 + 數字（預設 ⇧）：刪掉當前頁第幾個候選。不在組句中不管；那格沒有候選就吞掉按鍵不動。
    /// 刪完重新查一遍（排序會變），結果那句話顯示在拼音行右側，敲下一鍵就沒了。
    pub(super) fn handle_delete_key(&self, digit: usize, client: TextClient<'_>) -> bool {
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            return false;
        }
        let Some(message) = host::with(|h| h.forget_candidate(digit - 1)).flatten() else {
            tracing::debug!(digit, "这一格没有候选，没什么可删");
            return true;
        };
        tracing::info!(%message);
        self.refresh(client);
        host::with(|h| h.status = Some(message));
        self.render(client);
        true
    }
}
