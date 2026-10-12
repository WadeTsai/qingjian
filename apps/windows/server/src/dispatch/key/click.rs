//! 滑鼠點候選視窗：點候選與按它的數字鍵一樣上屏，點譯詞與按譯詞鍵一樣上屏那一條。
//! Server 不能直接寫文件：上屏的文本掛到焦點會話上，DLL 下一拍輪詢時隨 `Update.commit` 取走寫進去，
//! 趕在輪詢前又敲了鍵就接在那次按鍵結果前面。只對 v8 起的 DLL 收點選，更老的讀不到 `commit`。

use qingjian_platform::protocol::SessionId;
use qingjian_render::HitTarget;

use crate::dispatch::Router;

/// 從這一版協議起 DLL 會把 `Update.commit` 寫進文件。
const CLICK_SINCE: u32 = 8;

impl Router {
    /// 候選視窗上點中了候選或譯詞（UI 執行緒經工人通道送來）。
    pub fn handle_click(&mut self, target: HitTarget) {
        let Some(session) = self.focused else {
            return;
        };
        if self.focused_dll_protocol() < CLICK_SINCE {
            tracing::debug!("DLL 協議太老，不收候選視窗的點選");
            return;
        }
        if self.translation.is_some() || !self.composing() {
            return;
        }
        let text = match target {
            HitTarget::Candidate(offset) => self
                .slot_index(offset + 1)
                .and_then(|index| self.commit_index(index)),
            HitTarget::Translation { candidate, sense } => self
                .annotated_candidate_on_page(candidate + 1)
                .and_then(|candidate| self.engine.commit_translation(&candidate, sense)),
        };
        let Some(text) = text else {
            tracing::debug!(?target, "點中的格子沒有可上屏的");
            return;
        };
        tracing::debug!(?target, "點選上屏");
        let pending = self.take_clicked(session).unwrap_or_default();
        self.clicked = Some((session, pending + &text));
        self.recompose();
        let shown = self.self_drawn_frame();
        self.reconcile_candidates(&shown);
    }

    /// 取走這個會話點選上屏、還沒交出去的文本。
    pub(in crate::dispatch) fn take_clicked(&mut self, session: SessionId) -> Option<String> {
        if self
            .clicked
            .as_ref()
            .is_some_and(|(owner, _)| *owner == session)
        {
            self.clicked.take().map(|(_, text)| text)
        } else {
            None
        }
    }
}
