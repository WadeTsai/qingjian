//! 按訊息型別分派：會話開關、按鍵、輪詢、失焦上屏、選區 / 游標矩形 / 中英模式的通知。

use qingjian_platform::protocol::{
    ClientMessage, Frame, KeyEvent, KeyOutcome, PROTOCOL_VERSION, SESSION_OPENED_SINCE,
    ServerMessage, SessionId,
};

use super::Router;
use super::key::Effect;
use super::session::SessionInfo;

impl Router {
    pub(super) fn dispatch(&mut self, message: ClientMessage) -> Option<ServerMessage> {
        match message {
            ClientMessage::OpenSession {
                session,
                app,
                protocol,
            } => {
                tracing::debug!(?session, app, protocol, "會話開啟");
                if protocol != PROTOCOL_VERSION {
                    tracing::warn!(
                        ?session,
                        app,
                        dll = protocol,
                        server = PROTOCOL_VERSION,
                        "DLL 與 Server 的協議版本不同（應用還沒重啟、用著舊 DLL？），照常服務"
                    );
                }
                // 同一會話重開（DLL 斷線重連）：從乾淨狀態起。
                if self.focused == Some(session) {
                    self.reset_composition();
                    self.focused = None;
                }
                self.sessions.insert(
                    session,
                    SessionInfo {
                        app,
                        private: false,
                        protocol,
                        traditional: false,
                        zhuyin: false,
                    },
                );
                // 按鍵行為設定回一次，讓 DLL 不必自己讀配置檔案。**只回給會讀這條回包的 DLL**：
                // 更老的 DLL 的 `open` 是隻寫不讀，多回一條會被它當成下一次 `Poll` 的應答而報錯，
                // 那條連線就廢了（老 DLL 在沒重啟的應用裡還會活很久）。它們從 `ModeSync` 那一拍
                // 也能拿到同一份（新欄位它直接忽略），只是慢一拍。
                (protocol >= SESSION_OPENED_SINCE).then(|| ServerMessage::SessionOpened {
                    session,
                    input: self.input_settings(session),
                })
            }
            ClientMessage::Key { session, event } => Some(self.handle_key(session, event)),
            ClientMessage::Poll { session } => Some(self.handle_poll(session)),
            ClientMessage::Commit { session } => {
                let text = self.commit_raw_for(session);
                tracing::debug!(?session, ?text, "焦點離開，結束組句");
                Some(ServerMessage::Committed { session, text })
            }
            ClientMessage::Surrounding { session, text } => {
                tracing::trace!(?session, chars = text.chars().count(), "收到游標前文");
                self.set_surrounding(session, text);
                None
            }
            ClientMessage::Privacy { session, private } => {
                tracing::debug!(?session, private, "輸入框私密狀態");
                self.set_privacy(session, private);
                None
            }
            ClientMessage::Selection {
                session,
                request,
                text,
                rect,
            } => Some(self.handle_selection(session, request, text, rect)),
            ClientMessage::PositionCandidates { session, rect } => {
                self.position_candidates(session, rect);
                None
            }
            ClientMessage::HideCandidates { session } => {
                // 組句在 DLL 側結束（應用終止組句 / 翻譯評審失焦）：只收視窗；緩衝留給下一鍵的 Commit 清。
                if self.focused == Some(session) {
                    self.end_translation();
                    self.hide_candidate_window();
                }
                None
            }
            ClientMessage::ModeChanged { session, english } => {
                tracing::debug!(?session, english, "中英模式");
                self.handle_mode_changed(english);
                None
            }
            ClientMessage::SyncMode { session } => {
                self.handle_ime_active();
                Some(ServerMessage::ModeSync {
                    session,
                    english: Some(self.english),
                    input: self.input_settings(session),
                    indicator: self.indicator_state(),
                })
            }
            ClientMessage::ImeSwitched { session } => {
                tracing::debug!(?session, "切成了別的輸入法");
                self.handle_ime_switched();
                None
            }
            ClientMessage::Profile {
                session,
                traditional,
                zhuyin,
            } => {
                tracing::debug!(?session, traditional, zhuyin, "語言 profile");
                self.set_profile(session, traditional, zhuyin);
                None
            }
            ClientMessage::Indicator { session, command } => {
                tracing::debug!(?session, ?command, "工作列圖示選單");
                self.handle_indicator(command);
                None
            }
            ClientMessage::CloseSession { session } => {
                self.sessions.remove(&session);
                if self.focused == Some(session) {
                    self.reset_composition();
                    self.focused = None;
                }
                self.flush_learning();
                tracing::debug!(?session, "會話關閉");
                None
            }
        }
    }

    fn handle_key(&mut self, session: SessionId, event: KeyEvent) -> ServerMessage {
        self.ensure_focus(session);
        self.notice = None;
        if self.translation.is_some() {
            return self.handle_translation_review(session, &event);
        }
        if self.engine.composition().is_empty()
            && self.engine.prediction_enabled()
            && self.matches_translate_combo(&event)
        {
            self.selection_seq += 1;
            self.pending_selection = Some(self.selection_seq);
            tracing::debug!(
                ?session,
                request = self.selection_seq,
                "翻譯選中文字：請 DLL 讀選區"
            );
            return ServerMessage::RequestSelection {
                session,
                request: self.selection_seq,
            };
        }
        let (commit, outcome) = match self.apply_key(&event) {
            Effect::Changed(commit) => {
                self.recompose();
                (commit, KeyOutcome::Consumed)
            }
            Effect::Navigated => (None, KeyOutcome::Consumed),
            Effect::Passthrough => (None, KeyOutcome::Passthrough),
        };
        // 點選上屏的字還沒等到輪詢就來了鍵：接在這次結果前面，免得丟
        let commit = match (self.take_clicked(session), commit) {
            (Some(clicked), commit) => Some(clicked + commit.as_deref().unwrap_or_default()),
            (None, commit) => commit,
        };
        self.poll_prediction();
        // 自繪窗吃未降級的幀；發給 DLL 的那份按老協議降級（見 composed 的 current_frame）
        let shown = self.self_drawn_frame();
        self.reconcile_candidates(&shown);
        ServerMessage::KeyResult {
            session,
            outcome,
            commit,
            frame: self.current_frame(),
        }
    }

    /// 雲聯想輪詢：聚焦會話拉一次非同步結果回最新一幀，否則回空幀。釋義兜底與本地整句模型也借這個節拍收。
    fn handle_poll(&mut self, session: SessionId) -> ServerMessage {
        self.tick();
        let learned = self.engine.poll_glosses();
        if learned > 0 {
            tracing::info!(learned, "釋義兜底寫入個人釋義表");
        }
        let frame = if self.focused == Some(session) {
            self.poll_prediction();
            let shown = self.self_drawn_frame();
            self.reconcile_candidates(&shown);
            self.current_frame()
        } else {
            Frame::default()
        };
        ServerMessage::Update {
            session,
            frame,
            commit: self.take_clicked(session),
        }
    }
}
