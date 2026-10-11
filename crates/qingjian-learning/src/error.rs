#[derive(Debug, thiserror::Error)]
pub enum LearningError {
    /// 學習資料檔案讀寫失敗（許可權、壞盤一類；格式不對的行只跳過不報錯）。
    #[error("failed to read or write user learning file: {0}")]
    Io(#[from] std::io::Error),
}
