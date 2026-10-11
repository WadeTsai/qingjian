//! Linux 事件：釋放、焦點、能力與候選回撥不改 Windows 訊息格式。
use super::{Capabilities, DisplayIdentity};
use qingjian_platform::protocol::KeyEvent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LinuxEvent {
    Capabilities(Capabilities),

    Reset,

    Key {
        event: KeyEvent,

        release: bool,
    },

    Deactivate {
        focus_out: bool,

        client_preedit: bool,

        capability_changed: bool,
    },

    Focus {
        focused: bool,
    },

    Candidate {
        identity: DisplayIdentity,

        index: usize,
    },

    Page {
        identity: DisplayIdentity,

        next: bool,
    },
}
