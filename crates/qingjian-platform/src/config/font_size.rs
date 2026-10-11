//! 設定裡的字號（點）：0 或不是正數為「用主題的」，不限上限。包一層是為了讓配置能整體比較（`f32` 不是 `Eq`）。

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FontSize(pub f32);

impl FontSize {
    /// 填了的字號；沒填（0、負數、NaN）為 `None`。
    pub fn get(self) -> Option<f32> {
        (self.0.is_finite() && self.0 > 0.0).then_some(self.0)
    }

    /// 設定介面裡填的數；清空、負數、NaN 都是「用主題的」。
    pub fn from_input(size: Option<f64>) -> Self {
        Self(size.map_or(0.0, |size| size as f32))
    }

    /// 字號框改成 `input` 後該寫進配置的值；不用寫時為 `None`。
    /// 框裡沒設過時顯示的是主題的字號 `theme`，原樣交回來（失焦、初次顯示也會發值）不算改，免得把主題字號釘進配置。
    pub fn edited(self, input: Option<f64>, theme: Option<f32>) -> Option<Self> {
        let next = Self::from_input(input);
        let untouched = self.get().is_none() && next.get().is_some() && next.get() == theme;
        (next != self && !untouched).then_some(next)
    }
}

impl From<FontSize> for toml_edit::Value {
    /// 整數寫成整數（`16` 而不是 `16.0`），配置檔案裡好讀；沒填寫 0。
    fn from(size: FontSize) -> Self {
        let size = size.get().map_or(0.0, f64::from);
        if size.fract() == 0.0 && size < i64::MAX as f64 {
            (size as i64).into()
        } else {
            size.into()
        }
    }
}

impl PartialEq for FontSize {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_bits() == other.0.to_bits()
    }
}

impl Eq for FontSize {}
