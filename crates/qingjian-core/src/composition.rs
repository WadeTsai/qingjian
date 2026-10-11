//! 輸入狀態機：拼音緩衝區與游標。
//!
//! 只維護「使用者已經敲了什麼、游標在哪」，不理解拼音，也不知道候選。
//! 緩衝區只含 ASCII 小寫字母和 `'`（表示式模式下還有數字與運算子，見 `shortcut`；微軟 / 搜狗雙拼下還有 `;`），所以位元組下標即字元下標。
//! 中文模式下 Shift+字母按**小寫**進緩衝區參與匹配（`Cpan` 與 `cpan` 一樣出 C盤），
//! 敲的是大寫記在 `shifted` 裡，原樣上屏時用 [`Composition::typed_text`] 還原。

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Composition {
    /// 使用者已敲入、尚未上屏的拼音，匹配用的小寫形式。
    buffer: String,

    /// 與 `buffer` 的字元一一對應：該位是按住 Shift 敲的大寫字母。
    shifted: Vec<bool>,

    /// 游標位置（位元組下標，`0..=buffer.len()`），插入和退格都相對它。
    cursor: usize,
}

impl Composition {
    pub fn text(&self) -> &str {
        &self.buffer
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.cursor
    }

    /// 參與候選生成的那段拼音：游標停在中間時只算游標前的（`ni|hao` → `ni`），
    /// 游標在開頭或末尾時算整段。搜狗的行為：往回挪游標就是要先把前半段上屏。
    pub fn scope(&self) -> &str {
        if self.cursor == 0 || self.cursor >= self.buffer.len() {
            &self.buffer
        } else {
            &self.buffer[..self.cursor]
        }
    }

    /// 作用域之後剩下的拼音（`ni|hao` → `hao`），只用於顯示。
    pub fn rest(&self) -> &str {
        &self.buffer[self.scope().len()..]
    }

    /// 位元組位置 `byte`（須落在字元邊界上）對應的字元下標。
    fn char_index(&self, byte: usize) -> usize {
        self.buffer[..byte].chars().count()
    }

    /// 在游標處插入一個字元，大小寫原樣保留。
    pub fn push(&mut self, c: char) {
        let index = self.char_index(self.cursor);
        self.buffer.insert(self.cursor, c);
        self.shifted.insert(index, false);
        self.cursor += c.len_utf8();
    }

    /// 中文模式下按住 Shift 敲的字母：按小寫進緩衝區參與匹配，原樣上屏時還原大寫。
    pub fn push_shifted(&mut self, c: char) {
        let index = self.char_index(self.cursor);
        self.push(c.to_ascii_lowercase());
        if let Some(shifted) = self.shifted.get_mut(index) {
            *shifted = true;
        }
    }

    /// 緩衝區裡有沒有 Shift 敲的大寫字母。
    pub fn has_shifted(&self) -> bool {
        self.shifted.iter().any(|shifted| *shifted)
    }

    /// 緩衝區原樣（`shifted` 的位置還原大寫），回車原樣上屏用。
    pub fn typed_text(&self) -> String {
        self.typed(self.buffer.len())
    }

    /// [`Self::scope`] 的原樣形式。
    pub fn typed_scope(&self) -> String {
        self.typed(self.scope().len())
    }

    /// 前 `len` 位元組的原樣形式。
    fn typed(&self, len: usize) -> String {
        if !self.has_shifted() {
            return self.buffer[..len].to_owned();
        }
        self.buffer[..len]
            .chars()
            .enumerate()
            .map(|(index, c)| {
                if self.shifted.get(index) == Some(&true) {
                    c.to_ascii_uppercase()
                } else {
                    c
                }
            })
            .collect()
    }

    /// 刪掉游標前一個字元。游標在開頭時返回 `false`。
    pub fn backspace(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        let index = self.char_index(self.cursor) - 1;
        self.cursor -= 1;
        self.buffer.remove(self.cursor);
        self.shifted.remove(index);
        true
    }

    /// 刪掉游標後一個字元。游標在末尾時返回 `false`。
    pub fn delete_forward(&mut self) -> bool {
        if self.cursor >= self.buffer.len() {
            return false;
        }
        let index = self.char_index(self.cursor);
        self.buffer.remove(self.cursor);
        self.shifted.remove(index);
        true
    }

    /// 游標左移一格。已在開頭時返回 `false`。
    pub fn move_left(&mut self) -> bool {
        if self.cursor == 0 {
            return false;
        }
        self.cursor -= 1;
        true
    }

    /// 游標右移一格。已在末尾時返回 `false`。
    pub fn move_right(&mut self) -> bool {
        if self.cursor >= self.buffer.len() {
            return false;
        }
        self.cursor += 1;
        true
    }

    /// 刪掉游標前 `len` 個位元組（不夠就刪到開頭）。游標在開頭或 `len` 為零時返回 `false`。
    pub fn delete_before_cursor(&mut self, len: usize) -> bool {
        let len = len.min(self.cursor);
        if len == 0 {
            return false;
        }
        let from = self.char_index(self.cursor - len);
        let to = self.char_index(self.cursor);
        self.buffer.drain(self.cursor - len..self.cursor);
        self.shifted.drain(from..to);
        self.cursor -= len;
        true
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.buffer.len();
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.shifted.clear();
        self.cursor = 0;
    }

    /// 刪掉開頭 `len` 個位元組（上屏消耗掉的拼音），游標隨之前移；
    /// 被吃掉的部分蓋過了游標（`ni|hao` 上屏了 你）時游標落到末尾，接著組句就是往後打。
    pub fn drain_prefix(&mut self, len: usize) {
        let len = len.min(self.buffer.len());
        let removed = self.char_index(len);
        self.buffer.drain(..len);
        self.shifted.drain(..removed);
        self.cursor = if self.cursor > len {
            self.cursor - len
        } else {
            self.buffer.len()
        };
    }

    /// 吃掉整個作用域（雲端詞 / 英文詞 / 整句對應的是整個作用域）。
    pub fn drain_scope(&mut self) {
        let len = self.scope().len();
        self.drain_prefix(len);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn typed(text: &str) -> Composition {
        let mut composition = Composition::default();
        for c in text.chars() {
            composition.push(c);
        }
        composition
    }

    #[test]
    fn push_and_backspace() {
        let mut composition = Composition::default();
        // 原樣保留大小寫（英文直輸段 `no-Way` 要用），中文模式下殼只會送小寫
        composition.push('K');
        composition.push('a');
        assert_eq!(composition.text(), "Ka");
        assert_eq!(composition.cursor(), 2);
        assert!(composition.backspace());
        assert!(composition.backspace());
        assert!(!composition.backspace());
        assert!(composition.is_empty());
    }

    #[test]
    fn shifted_letters_match_lowercase_and_restore_case_on_raw_commit() {
        let mut composition = Composition::default();
        composition.push_shifted('C');
        composition.push('p');
        composition.push('a');
        composition.push('n');
        // 匹配用的小寫形式
        assert_eq!(composition.text(), "cpan");
        assert_eq!(composition.scope(), "cpan");
        // 原樣上屏用的大小寫
        assert_eq!(composition.typed_text(), "Cpan");
        assert_eq!(composition.typed_scope(), "Cpan");
        // 退格把後面的小寫刪掉，大寫那一位還留著
        for _ in 0..3 {
            composition.backspace();
        }
        composition.push('b');
        assert_eq!(composition.text(), "cb");
        assert_eq!(composition.typed_text(), "Cb");
        composition.clear();
        assert_eq!(composition.typed_text(), "");
        assert!(!composition.has_shifted());
    }

    #[test]
    fn edits_happen_at_the_cursor() {
        let mut composition = typed("kaifa");
        assert!(composition.move_left());
        assert!(composition.move_left());
        composition.push('n');
        assert_eq!(composition.text(), "kainfa");
        assert_eq!(composition.cursor(), 4);
        assert!(composition.backspace());
        assert_eq!(composition.text(), "kaifa");
        assert!(composition.delete_forward());
        assert_eq!(composition.text(), "kaia");
        composition.move_home();
        assert!(!composition.move_left());
        assert!(!composition.backspace());
        composition.move_end();
        assert!(!composition.move_right());
        assert!(!composition.delete_forward());
    }

    #[test]
    fn delete_before_cursor_stops_at_the_start() {
        let mut composition = typed("kaifa");
        composition.move_left();
        assert!(composition.delete_before_cursor(2));
        assert_eq!(composition.text(), "kaa");
        assert_eq!(composition.cursor(), 2);
        assert!(composition.delete_before_cursor(5));
        assert_eq!(composition.text(), "a");
        assert_eq!(composition.cursor(), 0);
        assert!(!composition.delete_before_cursor(1));
        assert!(!composition.delete_before_cursor(0));
    }

    #[test]
    fn scope_is_text_before_cursor_unless_cursor_is_at_either_end() {
        let mut composition = typed("nihao");
        assert_eq!(composition.scope(), "nihao");
        assert_eq!(composition.rest(), "");
        composition.move_left();
        composition.move_left();
        composition.move_left();
        assert_eq!(composition.scope(), "ni");
        assert_eq!(composition.rest(), "hao");
        composition.move_home();
        assert_eq!(composition.scope(), "nihao");
        assert_eq!(composition.rest(), "");
    }

    #[test]
    fn draining_past_the_cursor_moves_it_to_the_end() {
        let mut composition = typed("nihao");
        for _ in 0..3 {
            composition.move_left();
        }
        composition.drain_scope();
        assert_eq!(composition.text(), "hao");
        assert_eq!(composition.cursor(), 3);
    }

    #[test]
    fn drain_prefix_moves_cursor_back() {
        let mut composition = typed("kaifazhe");
        composition.move_left();
        composition.drain_prefix(5);
        assert_eq!(composition.text(), "zhe");
        assert_eq!(composition.cursor(), 2);
        composition.drain_prefix(10);
        assert!(composition.is_empty());
        assert_eq!(composition.cursor(), 0);
    }
}
