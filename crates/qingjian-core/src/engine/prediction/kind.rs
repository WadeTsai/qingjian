/// 一次聯想請求要的是什麼。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PredictionKind {
    /// 組句中：這段拼音對應的詞，可能還要整句補全。
    #[default]
    Compose,

    /// 問字模式（`?` 開頭）：用拼音問「三個木是什麼字」之類的問題，要答案（字或短答案）與讀音。
    Question,

    /// 翻譯：把應用裡選中的一段文字譯成學習語言（殼裡快捷鍵觸發），譯文放在結果的 `sentence` 裡。
    Translate,
}
