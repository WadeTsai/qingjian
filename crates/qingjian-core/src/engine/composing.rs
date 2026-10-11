//! 緩衝區與模式：按鍵進出、游標移動、表示式 / 英文直輸 / 問字等模式判斷、標點與上屏鏈。

use super::commit::LastCommit;
use super::input_log::{INPUT_LOG_VERSION, InputLogEntry, InputLogger, InputSource};
use super::learning::Learner;
use super::mode_keys::QUESTION_PREFIX;
use super::{Engine, RECENT_COMMITS, is_raw, looks_like_english_word, segment_longest_prefix};
use crate::composition::Composition;
use crate::shortcut;
use std::time::Instant;

/// 直通字元攢到這麼多就先寫一條，免得長時間純英文輸入時一條攢得沒邊。
const MAX_PENDING_PASSTHROUGH: usize = 200;

impl Engine {
    /// 中文模式下把半形字元轉成全形標點；不需要轉換返回 `None`。
    pub fn punctuate(&mut self, c: char) -> Option<&'static str> {
        // 組句外敲的標點：輔碼態到此結束（殼已經把高亮候選上屏了）
        self.aux_code = None;
        let converted = if self.full_width_punctuation {
            self.punctuation.convert(c)
        } else {
            None
        };
        if let Some(text) = converted {
            self.history.record(text);
            self.remember_commit(LastCommit::plain(text));
            // 全形標點也是文本流的一部分，與直通字元攢在一起
            self.passthrough_pending.push_str(text);
        } else {
            self.recent_commits.clear();
        }
        self.chain.reset();
        converted
    }

    /// 殼把字元原樣透傳給應用後告知，用於「數字後的點保持半形」，也記入輸入歷史與輸入日誌（攢成一條 `passthrough`）。
    pub fn note_passthrough(&mut self, c: char) {
        self.punctuation.note_passthrough(c);
        let text = c.encode_utf8(&mut [0; 4]).to_owned();
        self.history.record(&text);
        self.chain.reset();
        self.remember_commit(LastCommit::plain(&text));
        self.passthrough_pending.push(c);
        if self.passthrough_pending.chars().count() >= MAX_PENDING_PASSTHROUGH {
            self.flush_passthrough();
        }
    }

    /// 把攢著的直通字元寫成一條輸入日誌。上屏、上文斷開、會話記錄前都調，保證日誌裡的順序與真實順序一致。
    pub(super) fn flush_passthrough(&mut self) {
        if self.passthrough_pending.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.passthrough_pending);
        self.logger.record(InputLogEntry::Passthrough { text });
    }

    /// 殼告知游標離開了剛才上屏的位置（切換應用、點了別處、停用輸入法）：之後上屏的詞按句首記。
    /// 上次斷開之後有過上屏才往輸入日誌記一條 `break`，連著失焦幾次只記一次。
    pub fn break_chain(&mut self) {
        self.chain.reset();
        self.recent_commits.clear();
        self.flush_passthrough();
        if self.committed_since_break {
            self.committed_since_break = false;
            self.logger.record(InputLogEntry::Break {
                app: self.application.clone(),
            });
        }
    }

    /// 殼告知正在輸入的應用（macOS bundle identifier / Windows exe 名），寫進輸入日誌；不知道就給 `None`。
    pub fn set_application(&mut self, app: Option<String>) {
        self.application = app;
    }

    pub fn application(&self) -> Option<&str> {
        self.application.as_deref()
    }

    /// 殼翻了一頁候選：記進這段組句的翻頁數（寫進輸入日誌，候選質量的隱式訊號）。
    pub fn note_page_turn(&mut self) {
        self.page_turns = self.page_turns.saturating_add(1);
    }

    /// 往輸入日誌記一條會話資訊（版本、平臺、本地模型開沒開）。殼在啟動和開啟日誌時調。
    pub fn log_session(&mut self, version: &str, platform: &str) {
        self.flush_passthrough();
        self.logger.record(InputLogEntry::Session {
            v: INPUT_LOG_VERSION,
            version: version.to_owned(),
            platform: platform.to_owned(),
            model: self.has_sentence_scorer(),
            scheme: self.scheme_key(),
        });
    }

    /// 鍵盤方案的鍵，輸入日誌用。全拼為空串（老日誌裡沒有這個欄位就是全拼），雙拼是 `xiaohe` 這類，
    /// 注音是 `zhuyin`，只用形碼是 `wubi`，**混輸是 `<拼音側>+wubi`**（`pinyin+wubi` / `xiaohe+wubi`）。
    ///
    /// 日誌裡必須能分辨這幾種：形碼那些行的「拼音」列其實是編碼，回放要照著它裝配引擎，
    /// 混輸的行兩邊都要裝配。`wubi` 單獨出現是「只用形碼」，不是「全拼 + 五筆」。
    pub(super) fn scheme_key(&self) -> String {
        let phonetic = if self.zhuyin.is_some() {
            "zhuyin".to_owned()
        } else {
            self.shuangpin
                .map_or_else(String::new, |s| s.key().to_owned())
        };
        if self.code.is_none() {
            return phonetic;
        }
        if !self.phonetic {
            return "wubi".to_owned();
        }
        let base = if phonetic.is_empty() {
            "pinyin"
        } else {
            &phonetic
        };
        format!("{base}+wubi")
    }

    /// 組句裡要刪東西了：第一次刪之前把緩衝區留個快照，上屏時對比最終鍵串，不同就是一次重打（`retype`）。
    fn note_edit(&mut self) {
        if self.retype_snapshot.is_none() && !self.composition.is_empty() {
            self.retype_snapshot = Some(self.composition.text().to_owned());
        }
    }

    /// 殼告知：不在組句時按了退格，刪的是應用裡剛上屏的文字。從最近一次上屏往前數，一次上屏的字刪光了就是「可能選錯了」的訊號：
    /// 接著重打那段拼音選了別的詞，那次記的學習就退回去（見 [`Self::apply_retraction`]）。
    /// 刪得比記著的幾次上屏加起來還多說明在改別處，全忘掉。
    pub fn note_backspace(&mut self) {
        let Some(commit) = self
            .recent_commits
            .iter_mut()
            .rev()
            .find(|c| !c.is_erased())
        else {
            self.recent_commits.clear();
            self.chain.reset();
            return;
        };
        commit.erased += 1;
        if commit.is_erased() {
            // 剛上屏的詞沒了，它不再是下一個詞的上文
            self.chain.reset();
        }
    }

    /// 記一次上屏到最近上屏列表，超出條數丟最早的。
    pub(super) fn remember_commit(&mut self, commit: LastCommit) {
        if commit.chars == 0 {
            return;
        }
        if self.recent_commits.len() >= RECENT_COMMITS {
            self.recent_commits.remove(0);
        }
        self.recent_commits.push(commit);
    }

    pub fn composition(&self) -> &Composition {
        &self.composition
    }

    pub fn push(&mut self, c: char) {
        if self.composition.is_empty() {
            // 新一段組句：從這一鍵起算耗時、翻頁與重打
            self.composition_started = Some(Instant::now());
            self.page_turns = 0;
            self.retype_snapshot = None;
        }
        // 進緩衝區的字元也更新「上一個字元是不是數字」：直通了 `3` 再接著打拼音，
        // 後面的 `.` 不該仍按小數點保持半形（組句中標點會先把候選上屏，見殼的 apply_printable）
        self.punctuation.note_passthrough(c);
        // 中文模式下 Shift+字母（配置 `shift_letter = "compose"` 時才收）：按小寫進緩衝區參與匹配
        // （`Cpan` 與 `cpan` 一樣出 C盤），原樣上屏（回車 / 無候選）時再還原大寫。
        // 預設關：殼把大寫字母直接交給應用，根本進不到這裡；英文模式與英文直輸段（`no-Way`）始終保留原樣。
        if self.shift_letter_compose
            && c.is_ascii_uppercase()
            && !self.english_mode
            && !self.raw_mode()
        {
            self.composition.push_shifted(c);
        } else {
            self.composition.push(c);
        }
    }

    /// 退格。輔碼態裡刪的是碼段：刪掉最後一個碼字母；刪空時按「碼刪空後留在輔碼態」開關分岔——
    /// 開（預設）停在輔碼態（`;` 仍在、無碼詞也回來），關則回拼音態。碼段本來就空（剛觸發，或刪空停住）
    /// 時按退格 = 退出輔碼態、拼音一個字元都不動。每次退格候選都當場重篩。
    pub fn backspace(&mut self) -> bool {
        if let Some(code) = self.aux_code.take() {
            if code.len() > 1 {
                self.aux_code = Some(code[..code.len() - 1].to_owned());
            } else if code.len() == 1 && self.aux_keep_empty {
                self.aux_code = Some(String::new());
            }
            return true;
        }
        self.note_edit();
        self.composition.backspace()
    }

    pub fn clear(&mut self) {
        self.composition.clear();
        self.aux_code = None;
        self.chain.leave_buffer();
        // 殼給的游標前文只對這段組句有效，下一段第一鍵再讀
        self.rescoring_before = None;
        self.retype_snapshot = None;
        self.composition_started = None;
        self.page_turns = 0;
        self.traditional_map.borrow_mut().clear();
    }

    pub fn delete_forward(&mut self) -> bool {
        self.note_edit();
        self.composition.delete_forward()
    }

    /// 刪掉游標前的一個音節（殼裡 ⌥⌫）：全拼按最優切分的最後一個音節連同它後面的 `'`，切不動的尾巴整個刪；
    /// 雙拼兩鍵一音節，落單的一鍵單刪；英文直輸段 / 表示式 / 問字裡刪最後一段字母或數字，標點一次刪一個。
    /// 游標在開頭時返回 `false`。
    pub fn delete_syllable_backward(&mut self) -> bool {
        self.note_edit();
        let cursor = self.composition.cursor();
        let before = &self.composition.text()[..cursor];
        let plain = self.raw_mode()
            || self.expression_mode()
            || self.question_mode()
            || self.zhuyin.is_some();
        let len = unit_len_before(before, self.shuangpin.is_some(), plain);
        self.composition.delete_before_cursor(len)
    }

    /// 游標向左跳過一個音節，遇 `'` 連它一起跳過。游標在開頭時返回 `false`。
    pub fn move_cursor_syllable_left(&mut self) -> bool {
        let cursor = self.composition.cursor();
        let before = &self.composition.text()[..cursor];
        let plain = self.raw_mode()
            || self.expression_mode()
            || self.question_mode()
            || self.zhuyin.is_some();
        let len = unit_len_before(before, self.shuangpin.is_some(), plain);
        len > 0 && (0..len).all(|_| self.composition.move_left())
    }

    /// 游標向右跳過一個音節，遇 `'` 連它一起跳過。游標在末尾時返回 `false`。
    pub fn move_cursor_syllable_right(&mut self) -> bool {
        let cursor = self.composition.cursor();
        let after = &self.composition.text()[cursor..];
        let plain = self.raw_mode()
            || self.expression_mode()
            || self.question_mode()
            || self.zhuyin.is_some();
        let len = unit_len_after(after, self.shuangpin.is_some(), plain);
        len > 0 && (0..len).all(|_| self.composition.move_right())
    }

    /// 刪掉游標前的全部拼音（殼裡 ⌘⌫），游標後的留著。游標在開頭時返回 `false`。
    pub fn delete_to_start(&mut self) -> bool {
        self.note_edit();
        let cursor = self.composition.cursor();
        self.composition.delete_before_cursor(cursor)
    }

    pub fn move_cursor_left(&mut self) -> bool {
        self.composition.move_left()
    }

    pub fn move_cursor_right(&mut self) -> bool {
        self.composition.move_right()
    }

    pub fn move_cursor_home(&mut self) {
        self.composition.move_home();
    }

    pub fn move_cursor_end(&mut self) {
        self.composition.move_end();
    }

    /// 是否處在表示式模式（緩衝區以表示式鍵、預設 `v` 開頭）。此時殼應把數字和運算子也交給 [`Self::push`]，而不是當選詞鍵。
    pub fn expression_mode(&self) -> bool {
        !self.has_custom_phrase()
            && self
                .modes()
                .is_expression(self.composition.text(), self.zhuyin)
    }

    /// 英文直輸段：緩衝區裡有拼音以外的字元（`no-way`），整段原樣上屏、不解析拼音。
    /// 表示式模式與問字模式優先於它。
    pub fn raw_mode(&self) -> bool {
        is_raw(
            self.composition.text(),
            self.modes(),
            self.shuangpin,
            self.zhuyin,
        )
    }

    /// 是否處在問字模式（緩衝區以問字鍵、預設 `u`，或 `?` 開頭）：拼音問題由雲端答，十六進位制碼點本地答。
    pub fn question_mode(&self) -> bool {
        !self.has_custom_phrase()
            && self
                .modes()
                .is_question(self.composition.text(), self.zhuyin)
    }

    /// 問字模式下正在敲的還可能是 Unicode 碼點（字首後為空，或到目前為止全是十六進位制 / 開頭 `+`）：
    /// 此時殼應把數字交給 [`Self::push`] 而不是當選詞鍵。
    pub fn unicode_entry(&self) -> bool {
        let text = self.composition.text();
        self.modes().is_question(text, self.zhuyin)
            && shortcut::could_be_unicode(self.modes().question_body(text, self.zhuyin))
    }

    /// 緩衝區裡只有一個 `?`：還沒決定是問字還是標點。殼在確認標點時呼叫 [`Self::restore_bare_question`]。
    pub fn bare_question(&self) -> bool {
        self.composition.text() == QUESTION_PREFIX.to_string()
    }

    /// 確認單獨的問號並清空緩衝區；中文遵循標點設定，英文原樣輸出。
    /// 不是單獨的問號時返回 `None`，不改變組句；殼負責取消聯想介面並插入返回的文本。
    pub fn restore_bare_question(&mut self, english: bool) -> Option<String> {
        if !self.bare_question() {
            return None;
        }
        self.clear();
        if !english && let Some(mark) = self.punctuate(QUESTION_PREFIX) {
            return Some(mark.to_owned());
        }
        self.note_passthrough(QUESTION_PREFIX);
        Some(QUESTION_PREFIX.to_string())
    }

    /// 用一段完整拼音替換當前緩衝區，供 CLI 和測試一次性喂入。
    pub fn set_input(&mut self, input: &str) {
        self.composition.clear();
        for c in input.chars() {
            self.push(c);
        }
    }

    /// 放棄當前拼音，原樣返回給殼（通常是使用者按回車要上屏字母本身）。
    pub fn take_raw(&mut self) -> String {
        // 回車原樣上屏拼音段：碼段（沒上屏的碼）到此結束
        self.aux_code = None;
        // 糾錯生效時使用者仍按了回車：這個串就是要原樣打的，記下來以後不再糾它
        let scope = self.composition.scope().to_owned();
        if !self.english_mode && self.active_correction(&scope).is_some() {
            self.learner.record_raw(&scope);
            // 快取裡還是「要糾」，清掉讓下次重算
            *self.correction_cache.borrow_mut() = None;
        }
        let raw = self.raw_preedit().text;
        if raw.is_empty() {
            // 殼在回車 / 失焦時不管有沒有在組句都會來一趟：空的不記日誌、不計統計
            self.clear();
            self.chain.reset();
            return raw;
        }
        self.log_commit(&raw, &raw, InputSource::Raw);
        // 原樣上屏的是個英文詞（`gist`）：記進個人英文詞表，下次直接出候選。
        // 雙拼下全部鍵都能解成完整音節的（`nihc`）不是英文，是使用者要原樣打出雙拼鍵。
        // 整段能讀成「拼音頭 + 英文尾」的（`woxiangxuexirust`）不學：那是我們沒給對候選、使用者放棄了，
        // 把整串學成英文詞會反過來堵掉混輸那條路
        let english_word = looks_like_english_word(&raw, self.english_mode)
            && (self.english_mode || self.decode(&raw).is_none_or(|d| !d.is_complete()))
            && (self.english_mode || self.split_english_tail(&raw).is_none());
        if english_word {
            self.learner.learn_english(&raw);
        }
        self.meter_commit(&raw, InputSource::Raw, english_word);
        self.composition.clear();
        self.traditional_map.borrow_mut().clear();
        self.remember_commit(LastCommit::plain(&raw));
        self.punctuation.note_committed(&raw);
        self.history.record(&raw);
        self.chain.reset();
        raw
    }
}

/// 游標後的第一個「單位」佔幾個位元組：先跳過緊跟的 `'`，再算一個音節；規則同 [`unit_len_before`]。
fn unit_len_after(after: &str, shuangpin: bool, plain: bool) -> usize {
    let trimmed = after.trim_start_matches('\'');
    let separators = after.len() - trimmed.len();
    let Some(first) = trimmed.chars().next() else {
        return separators;
    };
    if plain || !first.is_ascii_lowercase() {
        let run = if first.is_ascii_alphanumeric() {
            trimmed
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .map(char::len_utf8)
                .sum()
        } else {
            first.len_utf8()
        };
        return separators + run;
    }
    if shuangpin {
        let run = trimmed
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || *c == ';')
            .count();
        return separators + run.min(2);
    }
    let syllable = match segment_longest_prefix(trimmed) {
        Ok((segmentations, _)) => segmentations
            .first()
            .and_then(|s| s.syllables.first())
            .map_or(1, |s| s.text.len()),
        Err(_) => 1,
    };
    separators + syllable
}

/// 游標前的最後一個「單位」佔幾個位元組：拼音裡是一個音節（連同它後面的 `'`），見 [`Engine::delete_syllable_backward`]。
fn unit_len_before(before: &str, shuangpin: bool, plain: bool) -> usize {
    let trimmed = before.trim_end_matches('\'');
    let separators = before.len() - trimmed.len();
    let Some(last) = trimmed.chars().last() else {
        return separators;
    };
    // 直輸段 / 表示式 / 問字，或末尾不是字母：字母數字連成一段刪，其他字元一次一個
    if plain || !last.is_ascii_lowercase() {
        let run = if last.is_ascii_alphanumeric() {
            trimmed
                .chars()
                .rev()
                .take_while(char::is_ascii_alphanumeric)
                .map(char::len_utf8)
                .sum()
        } else {
            last.len_utf8()
        };
        return separators + run;
    }
    if shuangpin {
        // 兩鍵一音節：連著的鍵數是奇數說明末尾落單一鍵
        let run = trimmed
            .chars()
            .rev()
            .take_while(|c| c.is_ascii_lowercase() || *c == ';')
            .count();
        return separators + if run % 2 == 1 { 1 } else { 2 };
    }
    let syllable = match segment_longest_prefix(trimmed) {
        Ok((_, tail)) if !tail.is_empty() => tail.len(),
        Ok((segmentations, _)) => segmentations
            .first()
            .and_then(|s| s.syllables.last())
            .map_or(1, |s| s.text.len()),
        Err(_) => 1,
    };
    separators + syllable
}
