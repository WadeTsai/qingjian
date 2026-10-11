//! 一條連線內的會話對映與獨立協商狀態。
use crate::protocol::DisplayIdentity;
use qingjian_platform::protocol::SessionId;

pub(super) struct Session {
    pub(super) global: SessionId,

    pub(super) identity: Option<DisplayIdentity>,

    pub(super) capabilities: bool,
}
