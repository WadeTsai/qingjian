//! 按鍵怎麼作用到 Engine / 高亮上。分流規則與 macOS 殼的 `handle_text` / `handle_command` 對齊。

use qingjian_core::{QUESTION_PREFIX, punctuation, shortcut};
use qingjian_platform::protocol::KeyEvent;

use super::{Effect, codes, with_prefix};
use crate::dispatch::Router;

impl Router {
    /// 功能鍵靠鍵碼，其餘靠字元。組句中修飾鍵 + 數字是快捷鍵；帶 Ctrl / Alt / Win 而沒配到快捷鍵的鍵歸應用。
    /// 表示式模式裡 Shift + 數字打的是 `^ * ( )`，不當快捷鍵。
    pub(crate) fn apply_key(&mut self, event: &KeyEvent) -> Effect {
        if self.composing()
            && !self.engine.expression_mode()
            && let Some(digit) = codes::digit_key(event.virtual_key)
            && let Some(effect) = self.apply_digit_shortcut(digit, event.modifiers.chord())
        {
            return effect;
        }
        if event.modifiers.has_command_key() {
            return Effect::Passthrough;
        }
        let Some(c) = event.character.filter(|c| !c.is_control()) else {
            return self.apply_function_key(event);
        };
        // Caps 亮著無論中英模式都直接出大寫英文；英文候選只在持久英文模式、Caps 滅、應用允許時給。
        let caps = event.modifiers.caps;
        let english = caps || event.modifiers.english_mode;
        let english_candidates = event.modifiers.english_mode
            && !caps
            && self.config.english_candidates_in(self.focused_app());
        // 緩衝區為空時敲 `?` 先進問字模式（配置 `[shortcut] question_mark`，預設關），中英文模式都行：
        // 後面跟字母就是在問字，跟別的鍵就還原成問號。
        if !self.composing() && c == QUESTION_PREFIX && self.engine.takes_question_mark() {
            self.engine.set_english_mode(false);
            self.engine.push(c);
            return Effect::Changed(None);
        }
        // 雙拼下 Shift+V / Shift+U 進表示式 / 問字模式（全拼下的 v / u 被音節佔了）。
        if !self.composing() && !english && self.engine.takes_mode_letter(c) {
            self.engine.set_english_mode(false);
            self.engine.push(c);
            return Effect::Changed(None);
        }
        let question = self.composing() && self.engine.question_mode();
        // 英文模式下問字：Caps 讓字母以大寫送來，按小寫收進問題。
        let c = if question && english && c.is_ascii_uppercase() {
            c.to_ascii_lowercase()
        } else {
            c
        };
        // 只有一個 `?` 時敲了字母以外的鍵：還原成問號上屏；空格只是「把這個 ? 上屏」，其他鍵按沒在組句重新分派。
        if question && !c.is_ascii_lowercase() && self.engine.bare_question() {
            let mark = self.restore_bare_question(english);
            if c == ' ' {
                return Effect::Changed(Some(mark));
            }
            return with_prefix(Some(mark), self.apply_key(event), c);
        }
        // 英文組詞中候選被關掉（Caps 亮 / 切應用）：敲過的字母先原樣上屏。
        let flushed = (self.composing() && !english_candidates && self.engine.english_mode())
            .then(|| self.engine.take_raw());
        self.engine
            .set_english_mode(english_candidates && !question);
        let effect = if english && !question {
            self.apply_english(c, english_candidates, event)
        } else {
            self.apply_chinese(c, event)
        };
        with_prefix(flushed, effect, c)
    }

    /// 緩衝區裡只有一個 `?`：清掉，還原成問號（按當前模式的全形設定轉）。
    fn restore_bare_question(&mut self, english: bool) -> String {
        self.engine.clear();
        if self.full_width_for(english)
            && let Some(mark) = self.engine.punctuate(QUESTION_PREFIX)
        {
            return mark.to_owned();
        }
        self.engine.note_passthrough(QUESTION_PREFIX);
        QUESTION_PREFIX.to_string()
    }

    /// 退格 / Esc / 回車 / Tab / 方向鍵；沒在組句時都交還應用。
    fn apply_function_key(&mut self, event: &KeyEvent) -> Effect {
        if !self.composing() {
            // 回車交給應用：文本流裡是一個段落邊界（macOS 殼同樣記）
            if event.virtual_key == codes::RETURN {
                self.engine.note_passthrough('\n');
            }
            return Effect::Passthrough;
        }
        // 只有一個 `?` 時按了回車：回車就是「把這個 ? 上屏」，吞掉，否則聊天框會連訊息一起發出去；
        // 退格 / Esc 照常刪掉它。其他功能鍵 macOS 殼還原後交給應用，Windows 放行同步、上屏非同步，
        // 先動游標再插問號會插錯位置，所以還原後一併吞掉。
        if self.engine.bare_question() && !matches!(event.virtual_key, codes::BACK | codes::ESCAPE)
        {
            let english = event.modifiers.caps || event.modifiers.english_mode;
            return Effect::Changed(Some(self.restore_bare_question(english)));
        }
        match event.virtual_key {
            codes::BACK => {
                self.engine.backspace();
                Effect::Changed(None)
            }
            codes::ESCAPE => {
                // 輔碼態裡 Esc 只清碼段、拼音留著（與 Core 的 clear_aux 語義一致）
                if self.engine.in_aux() {
                    self.engine.clear_aux();
                } else {
                    self.engine.clear();
                }
                Effect::Changed(None)
            }
            codes::RETURN => {
                if self.engine.is_zhuyin_mode() {
                    if event.modifiers.shift {
                        Effect::Changed(Some(self.engine.take_raw()))
                    } else {
                        Effect::Changed(Some(self.commit_highlighted()))
                    }
                } else {
                    Effect::Changed(Some(self.engine.take_raw()))
                }
            }
            codes::TAB if event.modifiers.shift => {
                self.page(-1);
                Effect::Navigated
            }
            codes::TAB if self.engine.english_mode() => {
                Effect::Changed(Some(self.commit_highlighted()))
            }
            // 中文模式 Tab：有整句補全就接受，否則下一頁。
            codes::TAB => match self.sentence.take() {
                Some(sentence) => Effect::Changed(Some(self.engine.accept_prediction(&sentence))),
                None => {
                    self.page(1);
                    Effect::Navigated
                }
            },
            codes::DOWN => {
                self.move_highlight(1);
                Effect::Navigated
            }
            codes::UP => {
                self.move_highlight(-1);
                Effect::Navigated
            }
            codes::NEXT => {
                self.page(1);
                Effect::Navigated
            }
            codes::PRIOR => {
                self.page(-1);
                Effect::Navigated
            }
            codes::LEFT => {
                self.engine.move_cursor_left();
                Effect::Changed(None)
            }
            codes::RIGHT => {
                self.engine.move_cursor_right();
                Effect::Changed(None)
            }
            codes::HOME => {
                self.engine.move_cursor_home();
                Effect::Changed(None)
            }
            codes::END => {
                self.engine.move_cursor_end();
                Effect::Changed(None)
            }
            _ => Effect::Passthrough,
        }
    }

    /// 中文模式：字母進拼音。預設 Shift 大寫是臨時打英文——組句中先把拼音原樣上屏、字母交給應用；
    /// 配 `[general] shift_letter = "compose"` 時大寫也收進緩衝區（Core 按小寫匹配、原樣上屏時還原大小寫）。
    /// 沒在組句時的其他字元走全形標點；組句中會轉全形的標點先上屏高亮候選再補標點，其餘符號仍進英文直輸段（見 apply_printable）。
    fn apply_chinese(&mut self, c: char, event: &KeyEvent) -> Effect {
        // 注音模式下數字與一部分標點（大千 `- ; , . /`，倚天還有 `' =`）就是鍵盤上的音節鍵，跟著進緩衝區；哪些鍵由排列定。
        let is_zhuyin_key = self.engine.is_zhuyin_key(c);
        if c.is_ascii_lowercase()
            || is_zhuyin_key
            || (c.is_ascii_uppercase() && self.config.shift_letter_compose)
        {
            // 輔碼態：字母進碼段（逐鍵即篩），不進拼音緩衝區
            if c.is_ascii_lowercase() && self.engine.push_aux_code(c) {
                return Effect::Changed(None);
            }
            // 大寫（shift_letter_compose）落在輔碼態：先清碼段回拼音態（與 Esc 同語義）再收進緩衝區；
            // 碼段不清會掛在已變化的緩衝區上繼續篩
            if c.is_ascii_uppercase() && self.engine.in_aux() {
                self.engine.clear_aux();
            }
            self.engine.push(c);
            return Effect::Changed(None);
        }
        if c.is_ascii_uppercase() {
            let raw = self.composing().then(|| self.engine.take_raw());
            self.engine.note_passthrough(c);
            return with_prefix(raw, Effect::Passthrough, c);
        }
        if !self.composing() {
            return self.apply_punctuation(c, event);
        }
        self.apply_printable(c, event)
    }

    /// 當前模式開著全形就讓 Core 轉（數字後的 `.` 與小鍵盤的鍵保持半形）；轉不了的原樣交給應用並告知 Core。
    fn apply_punctuation(&mut self, c: char, event: &KeyEvent) -> Effect {
        let english = event.modifiers.caps || event.modifiers.english_mode;
        if !codes::is_keypad(event.virtual_key)
            && self.full_width_for(english)
            && let Some(text) = self.engine.punctuate(c)
        {
            return Effect::Changed(Some(text.to_owned()));
        }
        self.engine.note_passthrough(c);
        Effect::Passthrough
    }

    /// 英文模式。開著候選：字母進緩衝區，選詞與中文模式一樣（空格選高亮、數字選當前頁第 N 個、翻頁鍵翻頁），
    /// 詞上屏後空格照樣交給應用；數字對應的格子沒有候選（詞表沒有的詞、候選不足 N 個）時是識別符號的一部分（`foo1`）。
    /// 回車 / 標點先把字母原樣上屏。關著候選：字母由我們插入（大小寫按 Shift）。
    /// 其他鍵按英文模式那份全形設定轉，轉不了的交給應用。
    fn apply_english(&mut self, c: char, candidates: bool, event: &KeyEvent) -> Effect {
        let composing = self.composing();
        if !candidates {
            let raw = composing.then(|| self.engine.take_raw());
            let effect = if c.is_ascii_alphabetic() {
                self.engine.note_passthrough(c);
                Effect::Changed(Some(c.to_string()))
            } else {
                self.apply_punctuation(c, event)
            };
            return with_prefix(raw, effect, c);
        }
        if composing
            && let Some(digit) = codes::digit(event)
            && let Some(index) = self.slot_index(digit)
        {
            return Effect::Changed(self.commit_index(index));
        }
        if c.is_ascii_alphabetic()
            || (composing && (c.is_ascii_digit() || matches!(c, '_' | '\'' | '-')))
        {
            self.engine.push(c);
            return Effect::Changed(None);
        }
        if composing && let Some(step) = codes::page_key(event, self.config.page_keys) {
            self.page(step);
            return Effect::Navigated;
        }
        let committed = composing.then(|| {
            if c == ' ' {
                self.commit_highlighted()
            } else {
                self.engine.take_raw()
            }
        });
        let effect = self.apply_punctuation(c, event);
        with_prefix(committed, effect, c)
    }

    /// 組句中的可列印鍵：數字選當前頁第 N 個（沒有這一格就進直輸段），翻頁鍵翻頁，空格上屏高亮，
    /// 會轉全形的標點上屏高亮候選再補標點，其餘符號進英文直輸段；已在直輸段裡就一律追加。
    /// 表示式模式（`v1+2`）裡數字和運算子進算式；問字模式敲的還可能是碼點（`u4e00`、`u+1f600`），數字與 `+` 進緩衝區；
    /// 微軟 / 搜狗雙拼的 `;` 是 ing 鍵，末尾有落單聲母時進緩衝區。
    fn apply_printable(&mut self, c: char, event: &KeyEvent) -> Effect {
        let expression = self.engine.expression_mode();
        if (expression && shortcut::is_expression_char(c))
            || (self.engine.unicode_entry() && (c.is_ascii_digit() || c == '+'))
            || (c == ';' && self.engine.takes_semicolon())
        {
            self.engine.push(c);
            return Effect::Changed(None);
        }
        // 英文直輸段（緩衝區裡已有 `-` 這類字元）：可見字元一律追加，數字與翻頁鍵也不再選詞 / 翻頁；
        // 空格整段原樣上屏，空格本身也要在（`hello, world`）。
        if self.engine.raw_mode() {
            if c == ' ' {
                let committed = self.commit_highlighted();
                self.engine.note_passthrough(c);
                return with_prefix(Some(committed), Effect::Passthrough, c);
            }
            if c.is_ascii_graphic() {
                self.engine.push(c);
                return Effect::Changed(None);
            }
        }
        // 輔碼觸發鍵：拼音打完整了、這個鍵也沒被鍵盤方案吃掉 → 進輔碼態（觸發鍵不進緩衝區）
        if self.engine.aux_trigger(c) {
            self.engine.enter_aux();
            return Effect::Changed(None);
        }
        // 已經在輔碼態裡再敲觸發鍵是冪等的，既不上屏候選也不當標點
        if self.engine.in_aux() && c == self.config.aux_code_key {
            return Effect::Changed(None);
        }
        // 碼段篩空（例如碼敲錯一個字母）：空格 / 標點 / 數字都不上屏原始拼音，吞掉停在輔碼態等退格
        if self.engine.in_aux() && !self.engine.aux_code().is_empty() && self.candidate_count() == 0
        {
            return Effect::Changed(None);
        }
        if let Some(digit) = codes::digit(event)
            && (!self.engine.is_zhuyin_mode() || self.navigated)
        {
            if let Some(index) = self.slot_index(digit) {
                return Effect::Changed(self.commit_index(index));
            }
            // 問字模式裡數字不是問題的一部分：沒有這一格就不算
            if self.engine.question_mode() {
                return Effect::Changed(None);
            }
        }
        if let Some(step) = codes::page_key(event, self.config.page_keys) {
            self.page(step);
            return Effect::Navigated;
        }
        if c == ' ' {
            if self.engine.zhuyin_needs_tone() {
                self.engine.push(c);
                return Effect::Changed(None);
            }
            return Effect::Changed(Some(self.commit_highlighted()));
        }
        // 表示式 / 問字模式下的其他字元不進緩衝區（與 macOS 殼一致），輔碼態裡敲標點同理（碼段隨之清空）；
        // 普通拼音組句裡的標點也一樣——會轉全形的（`,` `.` `?` `!` 引號、括號這些）先把高亮候選上屏再補標點
        //（`nihao,` 一氣打完「你好，」），翻頁鍵已在前面上方攔走，數字後的 `.` 是否保持半形由 Core 判斷；
        // 這條上屏行為由 `[general] punct_commits` 控制（預設關：標點進英文直輸段）。
        // 不會轉的符號（`-` `/` `@`）仍進英文直輸段；`'` 是隔音符，任何模式都進緩衝區。
        if (c != '\''
            && (expression
                || self.engine.question_mode()
                || (self.config.punct_commits && punctuation::converts(c))))
            || self.engine.in_aux()
        {
            let committed = self.commit_highlighted();
            let effect = self.apply_punctuation(c, event);
            return with_prefix(Some(committed), effect, c);
        }
        self.engine.push(c);
        Effect::Changed(None)
    }

    /// 數字鍵在當前頁對應的格子下標；這一頁沒有這一格（`gpt6` 只有三個候選）返回 `None`，數字當內容進緩衝區。
    /// 雲端詞還沒到的佔位格算有：按了不算，免得結果一到就選錯。
    pub(super) fn slot_index(&self, digit: usize) -> Option<usize> {
        let page_size = self.config.page_size;
        let index = self.highlight / page_size * page_size + digit - 1;
        (digit <= page_size && index < self.candidate_count()).then_some(index)
    }

    /// 上屏高亮候選；沒有候選時緩衝原樣上屏。
    fn commit_highlighted(&mut self) -> String {
        match self.commit_index(self.highlight) {
            Some(text) => text,
            None => self.engine.take_raw(),
        }
    }

    pub(super) fn composing(&self) -> bool {
        !self.engine.composition().is_empty()
    }
}
