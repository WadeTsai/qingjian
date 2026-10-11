use std::io::{Read, Write};

use qingjian_platform::protocol::{
    ClientMessage, Frame, IndicatorCommand, InputSettings, KeyEvent, PROTOCOL_VERSION, ScreenRect,
    ServerMessage, SessionId, read_message, write_message,
};

use super::{KeyReply, KeyResponse, ModeSyncReply};
use crate::error::ClientError;

/// 連 Server 的一個會話客戶端，開在一條已連好的雙工流上（Windows 下是命名管道，測試裡是記憶體流）。
/// 傳輸是一問一答；開關會話與通知類訊息單向發。
pub struct EngineClient<S> {
    stream: S,

    /// 本會話標識，隨每條訊息帶上。
    session: SessionId,

    /// 上次報給 Server 的私密狀態；`None` 是還沒報過（Server 按不私密起算）。
    private: Option<bool>,
}

impl<S: Read + Write> EngineClient<S> {
    /// 開一個會話；Server 隨即回一次按鍵行為設定（切換鍵、內建英文模式），帶出來交給呼叫方。
    /// `app` 是宿主應用的 exe 檔名，Server 據此查按應用的設定。
    pub fn open(
        mut stream: S,
        session: SessionId,
        app: Option<String>,
    ) -> Result<(Self, InputSettings), ClientError> {
        write_message(
            &mut stream,
            &ClientMessage::OpenSession {
                session,
                app,
                protocol: PROTOCOL_VERSION,
            },
        )?;
        let input = match read_message(&mut stream)?.ok_or(ClientError::Closed)? {
            ServerMessage::SessionOpened { input, .. } => input,
            _ => InputSettings::default(),
        };
        Ok((
            Self {
                stream,
                session,
                private: None,
            },
            input,
        ))
    }

    pub fn session(&self) -> SessionId {
        self.session
    }

    /// 在一條臨時流上通知 Server「本執行緒切成了別的輸入法」（狀態條收起）。不開會話、不回話。
    pub fn notify_ime_switched(mut stream: S, session: SessionId) -> Result<(), ClientError> {
        write_message(&mut stream, &ClientMessage::ImeSwitched { session })?;
        Ok(())
    }

    /// 送一個按鍵等結果。觸發「翻譯選中文字」快捷鍵時回 [`KeyReply::NeedSelection`]，
    /// 呼叫方須讀當前選區再用 [`Self::selection`] 回給 Server。
    pub fn key(&mut self, event: KeyEvent) -> Result<KeyReply, ClientError> {
        let message = ClientMessage::Key {
            session: self.session,
            event,
        };
        match self.call(&message)? {
            ServerMessage::KeyResult {
                outcome,
                commit,
                frame,
                ..
            } => Ok(KeyReply::Result(KeyResponse {
                outcome,
                commit,
                frame,
            })),
            ServerMessage::RequestSelection { request, .. } => {
                Ok(KeyReply::NeedSelection { request })
            }
            _ => Err(ClientError::Unexpected("expected key result")),
        }
    }

    /// 把讀到的選區發給 Server，等它回翻譯候選幀。空選區時 Server 不進入翻譯、回空幀。
    pub fn selection(
        &mut self,
        request: u64,
        text: String,
        rect: ScreenRect,
    ) -> Result<KeyResponse, ClientError> {
        let message = ClientMessage::Selection {
            session: self.session,
            request,
            text,
            rect,
        };
        match self.call(&message)? {
            ServerMessage::KeyResult {
                outcome,
                commit,
                frame,
                ..
            } => Ok(KeyResponse {
                outcome,
                commit,
                frame,
            }),
            _ => Err(ClientError::Unexpected("expected key result for selection")),
        }
    }

    /// 組句期間定時拉一次雲聯想的非同步結果，回最新一幀，外加滑鼠點候選視窗上屏的文本（沒點過為 `None`）。
    pub fn poll(&mut self) -> Result<(Frame, Option<String>), ClientError> {
        match self.call(&ClientMessage::Poll {
            session: self.session,
        })? {
            ServerMessage::Update { frame, commit, .. } => Ok((frame, commit)),
            _ => Err(ClientError::Unexpected("expected update for poll")),
        }
    }

    /// 讓 Server 清空緩衝，拿回要原樣上屏的文本（沒在組句時為 `None`）。
    pub fn commit(&mut self) -> Result<Option<String>, ClientError> {
        match self.call(&ClientMessage::Commit {
            session: self.session,
        })? {
            ServerMessage::Committed { text, .. } => Ok(text),
            _ => Err(ClientError::Unexpected("expected committed for commit")),
        }
    }

    /// 組句起始時把應用游標前的文字送給 Server（本地整句模型的前文）。不回話。
    pub fn surrounding(&mut self, text: String) -> Result<(), ClientError> {
        self.send(&ClientMessage::Surrounding {
            session: self.session,
            text,
        })
    }

    /// 起組句時報輸入框私密與否；與上次報的相同就不發（Server 預設按不私密）。不回話。
    pub fn set_private(&mut self, private: bool) -> Result<(), ClientError> {
        if self.private == Some(private) || (self.private.is_none() && !private) {
            self.private = Some(private);
            return Ok(());
        }
        self.send(&ClientMessage::Privacy {
            session: self.session,
            private,
        })?;
        self.private = Some(private);
        Ok(())
    }

    /// 報組句範圍的螢幕矩形，Server 據此擺候選視窗。不回話。
    pub fn position_candidates(&mut self, rect: ScreenRect) -> Result<(), ClientError> {
        self.send(&ClientMessage::PositionCandidates {
            session: self.session,
            rect,
        })
    }

    /// 問 Server 有沒有待處理的目標模式，順路取回最新的按鍵行為設定（每一拍都帶）。
    pub fn sync_mode(&mut self) -> Result<ModeSyncReply, ClientError> {
        match self.call(&ClientMessage::SyncMode {
            session: self.session,
        })? {
            ServerMessage::ModeSync {
                english,
                input,
                indicator,
                ..
            } => Ok(ModeSyncReply {
                english,
                input,
                indicator,
            }),
            _ => Err(ClientError::Unexpected("expected mode sync")),
        }
    }

    /// 把當前會話的中英模式推給 Server（懸浮狀態條）。不回話。
    pub fn mode_changed(&mut self, english: bool) -> Result<(), ClientError> {
        self.send(&ClientMessage::ModeChanged {
            session: self.session,
            english,
        })
    }

    /// 報本執行緒用的語言 profile 的輸入行為：是不是繁體（「中文(台灣)」下的青簡 / 青簡注音）、是不是注音（青簡注音）；
    /// Server 開會話時按簡體、配置方案算。不回話。
    pub fn profile(&mut self, traditional: bool, zhuyin: bool) -> Result<(), ClientError> {
        self.send(&ClientMessage::Profile {
            session: self.session,
            traditional,
            zhuyin,
        })
    }

    /// 工作列圖示右鍵選單裡點的項交給 Server。不回話。
    pub fn indicator(&mut self, command: IndicatorCommand) -> Result<(), ClientError> {
        self.send(&ClientMessage::Indicator {
            session: self.session,
            command,
        })
    }

    /// 讓 Server 收起候選視窗（組句在 DLL 側結束、Server 無從知曉時用）。不回話。
    pub fn hide_candidates(&mut self) -> Result<(), ClientError> {
        self.send(&ClientMessage::HideCandidates {
            session: self.session,
        })
    }

    /// 關閉會話，釋放 Server 側狀態。不回話。
    pub fn close(mut self) -> Result<(), ClientError> {
        self.send(&ClientMessage::CloseSession {
            session: self.session,
        })
    }

    fn send(&mut self, message: &ClientMessage) -> Result<(), ClientError> {
        write_message(&mut self.stream, message)?;
        Ok(())
    }

    /// 一問一答；對端在幀邊界關閉算 [`ClientError::Closed`]。
    fn call(&mut self, message: &ClientMessage) -> Result<ServerMessage, ClientError> {
        self.send(message)?;
        read_message(&mut self.stream)?.ok_or(ClientError::Closed)
    }
}
