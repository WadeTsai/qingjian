//! 候選的排布。與配置裡的 `LayoutMode` 一一對應，渲染器不依賴配置 crate，由殼換算。

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Layout {
    /// 豎排：一行一個候選，譯文在同一行右側。
    #[default]
    Vertical,

    /// 橫排：候選排成一行，只給高亮那個在下面顯示譯文；幀的 `columns` 不為 0 時展開成多行矩陣。
    Horizontal,
}
