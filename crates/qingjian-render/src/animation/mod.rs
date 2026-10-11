//! 動畫：緩動曲線、過渡的起止與插值。渲染器留住上一幀，按節點 `id` 配對算過渡，殼按 [`crate::Rendered::next_frame`] 定時要下一幀。

mod easing;
mod keyframes;
mod placement;
mod pose;
mod transition;

pub(crate) use easing::Easing;
pub(crate) use keyframes::Keyframes;
pub(crate) use placement::Placement;
pub(crate) use pose::Pose;
pub(crate) use transition::Transition;

use std::time::Duration;

/// 過渡在播時兩幀的間隔（約 60 幀）。
pub(crate) const FRAME_INTERVAL: Duration = Duration::from_millis(16);

/// 只有迴圈動畫在播時兩幀的間隔（上限 30 幀，省電）。
pub(crate) const LOOP_INTERVAL: Duration = Duration::from_millis(33);
