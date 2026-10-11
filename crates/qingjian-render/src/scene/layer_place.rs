//! 帶姿態節點的圖層按什麼算：父節點左上角與整張畫布大小（快取按它們對應，區域性重畫往小圖上畫時也不變）。

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LayerPlace {
    /// 父節點左上角（整張畫布畫素）。
    pub(crate) parent: (f32, f32),

    /// 整張畫布寬高。
    pub(crate) size: (u32, u32),
}
