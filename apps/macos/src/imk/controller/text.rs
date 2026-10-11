//! 可列印字元的處理：中英文模式、直輸段、表示式與問字模式的分流。

use super::*;

impl QingjianInputController {
    pub(super) fn handle_text(&self, text: &str, client: TextClient<'_>) -> bool {
        tracing::debug!(%text, "inputText");
        self.note_application(&client);
        let mut composing = host::with(|h| !h.engine.composition().is_empty()).unwrap_or(false);
        let english = modifiers::caps_lock_on();
        // 不在白名單（`[apps] english_candidates_on`）裡的應用，英文模式是純直通
        let english_candidates = english
            && host::with(|h| h.english_candidates_in(client.bundle_identifier().as_deref()))
                .unwrap_or(false);
        // 英文模式組詞中 Caps Lock 滅了（或開關關了）：敲的字母先原樣上屏，別把它們當拼音
        if composing
            && !english_candidates
            && host::with(|h| h.engine.english_mode()).unwrap_or(false)
        {
            self.commit_raw(client);
            composing = false;
        }
        let [byte] = text.as_bytes() else {
            // 多字元文本（如輸入法聯動、貼上）：先把當前候選（英文模式下是敲的字母）上屏，再交給應用
            if composing {
                if host::with(|h| h.engine.english_mode()).unwrap_or(false) {
                    self.commit_raw(client);
                } else {
                    self.commit_highlighted(client);
                }
            }
            return false;
        };
        let c = char::from(*byte);
        host::with(|h| h.indicator.update());
        // 緩衝區為空時敲 ? 先進問字模式（配置 `[shortcut] question_mark`，預設關），中英文模式都行：
        // 後面跟字母就是在問字，跟別的鍵就還原成問號
        if !composing
            && c == QUESTION_PREFIX
            && host::with(|h| h.engine.takes_question_mark()).unwrap_or(false)
        {
            host::with(|h| h.engine.push(c));
            self.refresh(client);
            return true;
        }
        // 雙拼下 Shift+V / Shift+U 進表示式 / 問字模式（全拼下的 v / u 被音節佔了）
        if !composing && !english && host::with(|h| h.engine.takes_mode_letter(c)).unwrap_or(false)
        {
            host::with(|h| h.engine.push(c));
            self.refresh(client);
            return true;
        }
        let question = composing && host::with(|h| h.engine.question_mode()).unwrap_or(false);
        // 英文模式下問字：Caps Lock 讓字母以大寫送來，按小寫收進問題
        let c = if question && english && c.is_ascii_uppercase() {
            c.to_ascii_lowercase()
        } else {
            c
        };
        host::with(|h| h.engine.set_english_mode(english_candidates && !question));
        let (page_previous, page_next) =
            host::with(|h| h.page_keys).unwrap_or(qingjian_platform::DEFAULT_PAGE_KEYS);
        // Caps Lock 亮著 = 英文模式：不組句、不轉標點，字母預設小寫、按住 Shift 才大寫
        if english && !question {
            // Caps Lock 亮著時 macOS 不管按沒按 Shift 送來的都是大寫，只能讀 Shift 狀態：按著才大寫
            let letter = if modifiers::shift_down() {
                c.to_ascii_uppercase()
            } else {
                c.to_ascii_lowercase()
            };
            if !english_candidates {
                if composing {
                    self.commit_raw(client);
                }
                if c.is_ascii_alphabetic() {
                    client.insert_text(&letter.to_string());
                    host::with(|h| h.engine.note_passthrough(letter));
                    return true;
                }
                host::with(|h| h.engine.note_passthrough(c));
                return false;
            }
            // 英文候選：字母（以及組詞中的 _ ' -）進緩衝區，候選來自英文詞表。選詞與中文模式一樣：
            // 空格選高亮（詞上屏後空格照樣交給應用，接著打下一個詞）、數字選當前頁第 N 個、翻頁鍵翻頁；
            // 數字對應的格子沒有候選（kubectl 這類詞表沒有的詞、候選不足 N 個）時是識別符號的一部分（foo1）。
            // 回車、標點先把敲的字母原樣上屏再交給應用
            if composing
                && let Some(offset) = c.to_digit(10).filter(|d| *d > 0)
                && let Some(index) =
                    host::with(|h| h.session.index_on_page(offset as usize - 1)).flatten()
            {
                return self.commit_index(index, client);
            }
            if c.is_ascii_alphabetic()
                || (composing && (c.is_ascii_digit() || matches!(c, '_' | '\'' | '-')))
            {
                host::with(|h| h.engine.push(letter));
                self.refresh(client);
                return true;
            }
            if composing && c == page_previous {
                return self.turn_page(-1, client);
            }
            if composing && c == page_next {
                return self.turn_page(1, client);
            }
            if composing {
                if c == ' ' {
                    self.commit_highlighted(client);
                } else {
                    self.commit_raw(client);
                }
            }
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        // 表示式模式（v 開頭）：數字與運算子進緩衝區，不當選詞 / 翻頁鍵
        let expression = composing && host::with(|h| h.engine.expression_mode()).unwrap_or(false);
        // 英文直輸段（緩衝區裡已有 `-` 這類字元）：可見字元一律追加，空格 / 回車整段原樣上屏
        let raw = composing && host::with(|h| h.engine.raw_mode()).unwrap_or(false);
        // 組句中敲 `-`：進入英文直輸段（`no-way`）；配成翻頁鍵（`[general] page_keys` 選 `-=`）時才翻頁
        let hyphen = composing && !question && c == '-' && c != page_previous && c != page_next;
        // 問字模式下敲的還可能是碼點（`u4e00`、`u+1f600`）：數字與 `+` 進緩衝區而不是選詞
        let unicode = question && host::with(|h| h.engine.unicode_entry()).unwrap_or(false);
        // 微軟 / 搜狗雙拼的 `;` 是 ing 鍵：末尾有落單聲母時進緩衝區，其他時候還是標點
        let semicolon =
            composing && c == ';' && host::with(|h| h.engine.takes_semicolon()).unwrap_or(false);
        // 組句中敲不會轉全形的符號（`/` `@`，`-` 由上面的 hyphen 接）：進緩衝區，整段成為英文直輸段（`a@b`），
        // 中文模式下也能打帶符號的英文；翻頁鍵除外；⇧+數字（! @ # …）在前面已被刪候選 / 譯詞鍵截走。
        // 會轉全形的標點（`,` `.` `?` `!` 引號、括號這些）不進緩衝區：落到下方先把高亮候選上屏再補全形標點
        //（`nihao,` 一氣打完「你好，」），與 Windows / Linux 殼一致；這條上屏行為由 `[general] punct_commits`
        // 控制（預設開），關掉恢復老行為：標點也進緩衝區（英文直輸段）。
        let punct_commits = host::with(|h| h.punct_commits).unwrap_or(false);
        let punctuation = composing
            && !question
            && !expression
            && c.is_ascii_punctuation()
            && c != page_previous
            && c != page_next
            && !(punct_commits && qingjian_core::punctuation::converts(c));
        if c.is_ascii_lowercase()
            || (composing && c == '\'')
            || semicolon
            || (expression && qingjian_core::shortcut::is_expression_char(c))
            || (raw && c.is_ascii_graphic())
            || (unicode && (c.is_ascii_digit() || c == '+'))
            || hyphen
            || punctuation
        {
            host::with(|h| h.engine.push(c));
            self.refresh(client);
            return true;
        }
        // 直輸段裡的空格：整段原樣上屏，空格本身也交給應用（`hello, world` 裡的空格要在）
        if raw && c == ' ' {
            self.commit_highlighted(client);
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        if composing && self.restore_bare_question(client) {
            // 空格只是「把這個 ? 上屏」，不再多打一個空格；其他鍵按非組句狀態繼續處理
            if c == ' ' {
                return true;
            }
            return self.handle_text(text, client);
        }
        // 按住 Shift 打的大寫字母：預設是臨時打英文，先把拼音原樣上屏，再把字母交給應用；
        // `[general] shift_letter = "compose"` 時進緩衝區（Core 按小寫匹配、原樣上屏時還原大寫）
        if c.is_ascii_uppercase() {
            if host::with(|h| h.engine.shift_letter_compose()).unwrap_or(false) {
                host::with(|h| h.engine.push(c));
                self.refresh(client);
                return true;
            }
            if composing {
                self.commit_raw(client);
            }
            host::with(|h| h.engine.note_passthrough(c));
            return false;
        }
        if composing {
            match c {
                ' ' => return self.commit_highlighted(client),
                '1'..='9' => {
                    let offset = usize::from(*byte - b'1');
                    if let Some(index) = host::with(|h| h.session.index_on_page(offset)).flatten() {
                        return self.commit_index(index, client);
                    }
                    // 這一頁沒有這一格（`gpt6` 只有三個候選）：數字當內容進緩衝區，成為英文直輸段；
                    // 問字模式裡數字不是問題的一部分，不算
                    if !question {
                        host::with(|h| h.engine.push(c));
                        self.refresh(client);
                    }
                    return true;
                }
                c if c == page_previous => return self.turn_page(-1, client),
                c if c == page_next => return self.turn_page(1, client),
                // 其他字元：把當前高亮候選上屏，再按非組句狀態處理這個字元
                _ => {
                    self.commit_highlighted(client);
                }
            }
        }
        // 中文模式下的全形標點；轉不了的（數字、字母以外的其他鍵）原樣交給應用
        match host::with(|h| h.engine.punctuate(c)).flatten() {
            Some(full_width) => {
                client.insert_text(full_width);
                true
            }
            None => {
                host::with(|h| h.engine.note_passthrough(c));
                false
            }
        }
    }
}
