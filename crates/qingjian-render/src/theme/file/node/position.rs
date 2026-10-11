//! 定位方式：預設參與父框佈局，`absolute` 不佔位、按 `inset` 相對父框定位。

use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Position {
    Absolute,
}
