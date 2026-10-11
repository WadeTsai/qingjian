//! 輔碼查詢介面：Core 只看這一個 trait，不認識碼錶檔案。

/// 輔碼查詢：Core 只認這個介面，不認識碼錶檔案。過濾是反向的：拿候選詞逐個問有沒有以碼段開頭的碼
/// （設計見 `docs/design/aux-code.md`）。
pub trait AuxCodeLookup: Send + Sync {
    /// `word` 的碼裡第一個以 `prefix` 開頭的，返回那條碼本身（顯示碼取的就是它）。
    ///
    /// 同一個詞有多條碼時取檔案裡的第一條；多張碼錶疊加時由呼叫方按表序取先命中的那張。
    /// 沒命中返回 `None`。`prefix` 為空時任何碼都算命中。
    fn code_with_prefix<'a>(&'a self, word: &str, prefix: &str) -> Option<&'a str>;

    /// `word` 的全部碼，按表裡的順序；沒有這個詞時為空。
    ///
    /// 只給「檢視碼」這類診斷用：過濾走 `code_with_prefix`，熱路徑上不必列舉。
    fn codes_of<'a>(&'a self, word: &str) -> Vec<&'a str>;
}
