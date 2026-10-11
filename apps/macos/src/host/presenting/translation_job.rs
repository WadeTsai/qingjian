use objc2_foundation::NSRange;

/// 一次「翻譯選中文字」：從按下快捷鍵到使用者接受或放棄。
#[derive(Debug, Clone)]
pub struct TranslationJob {
    /// 選區在應用裡的範圍，接受時用譯文替換它。
    pub range: NSRange,

    /// 雲端回來的譯文；`None` 表示還在等。
    pub result: Option<String>,
}
