//! 命令鍵（回車、退格、方向鍵、Tab、Esc 等）的處理。

use super::*;

impl QingjianInputController {
    /// 組句期間所有編輯動作都由我們接管；不認識的一律吞掉，否則應用會動游標、丟 marked text。
    pub(super) fn handle_command(&self, selector: Sel, client: TextClient<'_>) -> bool {
        tracing::debug!(selector = %selector, "didCommandBySelector");
        self.note_application(&client);
        let composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        if !composing {
            // 刪的是應用裡的文字：剛上屏的詞被整個刪掉是「選錯了」的訊號，Engine 記著；
            // 按詞 / 按行刪的數不清刪了幾個字，撤銷的賬就不記了
            if selector == sel!(deleteBackward:) {
                host::with(|h| h.engine.note_backspace());
            } else if selector == sel!(deleteWordBackward:)
                || selector == sel!(deleteToBeginningOfLine:)
            {
                host::with(|h| h.engine.break_chain());
            } else if selector == sel!(insertNewline:) {
                // 回車交給應用：文本流裡是一個段落邊界
                host::with(|h| h.engine.note_passthrough('\n'));
            }
            return false;
        }
        if selector == sel!(deleteBackward:) {
            host::with(|h| h.engine.backspace());
            self.refresh(client);
        } else if selector == sel!(deleteWordBackward:) {
            // ⌥⌫：刪游標前一個音節
            host::with(|h| h.engine.delete_syllable_backward());
            self.refresh(client);
        } else if selector == sel!(deleteToBeginningOfLine:) {
            // ⌘⌫：刪游標前的全部拼音
            host::with(|h| h.engine.delete_to_start());
            self.refresh(client);
        } else if selector != sel!(cancelOperation:)
            && selector != sel!(complete:)
            && self.restore_bare_question(client)
        {
            // 只有一個 ? 時按了回車：回車就是「把這個 ? 上屏」，吞掉，否則聊天框裡會連訊息一起發出去；
            // 方向鍵等其他鍵還原後交給應用
            return selector == sel!(insertNewline:);
        } else if selector == sel!(insertNewline:) {
            self.commit_raw(client);
        } else if selector == sel!(cancelOperation:) || selector == sel!(complete:) {
            // TextEdit 等應用把 Esc 綁成 complete:（自動補全），也當作取消。矩陣展開著時第一下 Esc 只收回單行
            if host::with(|h| h.session.collapse()).unwrap_or(false) {
                self.render(client);
                return true;
            }
            host::with(|h| {
                h.engine.clear();
                h.cancel_prediction();
            });
            self.refresh(client);
        } else if selector == sel!(insertTab:) {
            // 英文模式 Tab 選中高亮的詞；中文模式有整句補全時接受它，否則翻頁
            if host::with(|h| h.engine.english_mode()).unwrap_or(false) {
                self.commit_highlighted(client);
            } else if !self.accept_sentence(client) {
                self.turn_page(1, client);
            }
        } else if selector == sel!(deleteForward:) {
            host::with(|h| h.engine.delete_forward());
            self.refresh(client);
        } else if selector == sel!(moveDown:) {
            self.move_highlight(1, client);
        } else if selector == sel!(moveUp:) {
            self.move_highlight(-1, client);
        } else if selector == sel!(moveLeft:) {
            // 橫排矩陣開著：左右鍵在候選之間移動高亮（單行、展開後都是）；否則照舊移動拼音游標
            if !self.move_cells(-1, client) {
                host::with(|h| h.engine.move_cursor_left());
                self.refresh(client);
            }
        } else if selector == sel!(moveRight:) {
            if !self.move_cells(1, client) {
                host::with(|h| h.engine.move_cursor_right());
                self.refresh(client);
            }
        } else if selector == sel!(moveWordLeft:) {
            // ⌥←：游標往左跳一個音節
            host::with(|h| h.engine.move_cursor_syllable_left());
            self.refresh(client);
        } else if selector == sel!(moveWordRight:) {
            // ⌥→：游標往右跳一個音節
            host::with(|h| h.engine.move_cursor_syllable_right());
            self.refresh(client);
        } else if selector == sel!(moveToBeginningOfLine:) || selector == sel!(moveToLeftEndOfLine:)
        {
            host::with(|h| h.engine.move_cursor_home());
            self.refresh(client);
        } else if selector == sel!(moveToEndOfLine:) || selector == sel!(moveToRightEndOfLine:) {
            host::with(|h| h.engine.move_cursor_end());
            self.refresh(client);
        } else if selector == sel!(pageDown:) || selector == sel!(scrollPageDown:) {
            self.turn_page(1, client);
        } else if selector == sel!(pageUp:)
            || selector == sel!(scrollPageUp:)
            || selector == sel!(insertBacktab:)
        {
            self.turn_page(-1, client);
        }
        true
    }
}
