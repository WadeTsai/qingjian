/// 解碼出的一個單元：一兩個鍵對應的全拼。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unit {
    /// 敲的鍵（1 或 2 個字元；使用者自己敲的 `'` 單獨一個單元）。
    pub keys: String,

    /// 翻出來的全拼：兩鍵是完整音節，落單的一鍵是聲母（`v` → `zh`）或母音（`a`），`'` 為空。
    pub pinyin: String,

    /// 是否是完整音節（兩鍵）。
    pub complete: bool,
}

impl Unit {
    pub fn separator() -> Self {
        Self {
            keys: "'".to_owned(),
            pinyin: String::new(),
            complete: false,
        }
    }

    pub fn is_separator(&self) -> bool {
        self.pinyin.is_empty()
    }
}
