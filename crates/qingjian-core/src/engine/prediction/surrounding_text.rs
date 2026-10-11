/// 殼從應用讀到的游標附近文本。應用不支援時整個為 `None`，Engine 退回本地輸入歷史。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SurroundingText {
    /// 游標（或 marked text 起點）之前的文本，殼可以給得比觀察視窗長，Engine 會裁。
    pub before: String,

    /// marked text 之後的文本。
    pub after: String,
}
