use std::rc::Rc;

use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl,
};
use windows::core::{Ref, Result, implement};

use super::Shared;
use crate::com::log::log;

/// 應用強行結束我們的組句（如點到別處）時框架回調。拼音已被框架定成普通文本，
/// 只清本地狀態、記下 Server 緩衝要清，絕不能再把 Server 交出的文本插一次。
#[implement(ITfCompositionSink)]
pub(super) struct CompositionSink {
    shared: Rc<Shared>,
}

impl CompositionSink {
    pub(super) fn new(shared: Rc<Shared>) -> Self {
        Self { shared }
    }
}

impl ITfCompositionSink_Impl for CompositionSink_Impl {
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        _composition: Ref<ITfComposition>,
    ) -> Result<()> {
        log("組句被應用終止，清本地狀態");
        self.shared.terminated();
        Ok(())
    }
}
