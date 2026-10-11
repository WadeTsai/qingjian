/// 控制元件當前的值：勾選框是布林，彈出選單是選項下標，文本框是文本。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SettingValue {
    /// 勾選框。
    Bool(bool),

    /// 彈出選單選中項的下標。
    Index(usize),

    /// 文本框內容（未裁剪）。
    Text(String),
}
