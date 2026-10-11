//! 本地輸入歷史：本次會話裡經我們上屏的文本，按時間順序拼成一段。
//!
//! 只用於兩件事：應用不提供游標附近文本時給聯想當上下文；以後給個人模型當訓練資料。
//! 全部在記憶體裡，落盤由殼決定；容量有上限，超過就丟最舊的。

/// 記憶體裡最多保留多少個字元。夠聯想的觀察視窗用，也不會無限增長。
const DEFAULT_CAPACITY: usize = 4096;

#[derive(Debug, Clone)]
pub struct InputHistory {
    /// 已上屏文本，按時間順序首尾相接。
    text: String,

    /// 字元數上限。
    capacity: usize,
}

impl Default for InputHistory {
    fn default() -> Self {
        Self::with_capacity(DEFAULT_CAPACITY)
    }
}

impl InputHistory {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            text: String::new(),
            capacity,
        }
    }

    /// 追加一段上屏文本，超出容量時從頭丟棄。
    pub fn record(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        self.text.push_str(text);
        let count = self.text.chars().count();
        if count > self.capacity {
            let drop = count - self.capacity;
            let start = self
                .text
                .char_indices()
                .nth(drop)
                .map_or(self.text.len(), |(i, _)| i);
            self.text.drain(..start);
        }
    }

    /// 最近 `chars` 個字元。
    pub fn recent(&self, chars: usize) -> &str {
        if chars == 0 {
            return "";
        }
        let count = self.text.chars().count();
        if chars >= count {
            return &self.text;
        }
        let start = self
            .text
            .char_indices()
            .nth(count - chars)
            .map_or(0, |(i, _)| i);
        &self.text[start..]
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// 一鍵清除。
    pub fn clear(&mut self) {
        self.text.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_in_order_and_returns_recent_chars() {
        let mut history = InputHistory::default();
        history.record("我们");
        history.record("今天");
        history.record("");
        assert_eq!(history.text(), "我们今天");
        assert_eq!(history.recent(2), "今天");
        assert_eq!(history.recent(10), "我们今天");
        assert_eq!(history.recent(0), "");
    }

    #[test]
    fn drops_oldest_beyond_capacity_on_char_boundaries() {
        let mut history = InputHistory::with_capacity(3);
        history.record("开发");
        history.record("输入法");
        assert_eq!(history.text(), "输入法");
        history.record("a");
        assert_eq!(history.text(), "入法a");
        history.clear();
        assert!(history.is_empty());
    }
}
