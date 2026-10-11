//! 滑鼠點候選視窗：按點陣圖渲染器給的點選區域認出點中的候選或譯詞，交給 host 註冊的回撥上屏。
//! AppKit 逐項繪製的舊路徑沒有點選區域，點了不動。

use objc2::DefinedClass;
use objc2_app_kit::NSEvent;
use qingjian_render::HitTarget;

use super::CandidateView;

/// 點中候選或譯詞時調的回撥。
pub type ClickHandler = Box<dyn Fn(HitTarget)>;

impl CandidateView {
    pub fn set_click_handler(&self, handler: ClickHandler) {
        *self.ivars().on_click.borrow_mut() = Some(handler);
    }

    pub(super) fn clicked(&self, event: &NSEvent) {
        let point = self.convertPoint_fromView(event.locationInWindow(), None);
        // 先放掉點陣圖的借用：回撥裡上屏會重畫這個檢視
        let target = self
            .ivars()
            .bitmap
            .borrow()
            .as_ref()
            .and_then(|bitmap| bitmap.hit(point));
        let Some(target) = target else {
            return;
        };
        tracing::debug!(?target, "点击候选窗口");
        if let Some(handler) = &*self.ivars().on_click.borrow() {
            handler(target);
        }
    }
}
