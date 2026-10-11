//! 正體 / 斜體。字型沒有斜體面時由渲染器把正體傾斜 14° 代替。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum FontStyle {
    #[default]
    Normal,

    Italic,
}
