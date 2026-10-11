//! 候選視窗輸出：Router 只產出 [`Frame`] 與游標矩形，交給 [`CandidateSink`] 去畫；幀沒變就不重畫。

mod sink;

use qingjian_platform::protocol::{Frame, ScreenRect, SessionId};
use qingjian_platform::scheme_name;
use qingjian_render::Mode;

pub use self::sink::{CandidateSink, NoopSink, RenderSettings};
use super::Router;

impl Router {
    /// 空幀收視窗；非空且已知游標矩形就重繪；還沒收到矩形（組句剛起）先不顯示，免得在舊位置閃一下。
    pub(super) fn reconcile_candidates(&mut self, frame: &Frame) {
        if frame.is_empty() {
            self.engine.note_displayed(std::iter::empty());
            self.hide_candidate_window();
        } else if let Some(rect) = self.last_rect {
            let mode = self.input_mode();
            let unchanged = matches!(&self.last_shown,
                Some((f, m, r)) if f == frame && *m == mode && *r == rect);
            if !unchanged {
                // 詞彙記錄的「看到輪次」按真正顯示的頁算，與 macOS 殼對齊。
                self.engine.note_displayed(frame.candidates.items.iter());
                self.candidates.show(frame.clone(), mode.clone(), rect);
                self.last_shown = Some((frame.clone(), mode, rect));
            }
        }
    }

    /// 給主題顯示的輸入狀態，候選視窗與狀態條共用。
    pub(super) fn input_mode(&self) -> Mode {
        Mode {
            english: self.english,
            traditional: self.traditional_output(),
            full_width: self.full_width_for(self.english),
            scheme: scheme_name(self.config.scheme, self.config.wubi),
        }
    }

    pub(super) fn hide_candidate_window(&mut self) {
        self.last_rect = None;
        self.last_shown = None;
        self.candidates.hide();
    }

    pub(super) fn position_candidates(&mut self, session: SessionId, rect: ScreenRect) {
        if self.focused != Some(session) {
            return;
        }
        self.last_rect = Some(rect);
        // 自繪窗吃未降級的幀（降級只作用於發給 DLL 的那份）
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
    }
}
