//! 本地整句模型（與 macOS 殼的 `host/model.rs` 對齊）：後臺載入、停鍵後請求重排、結果到了重畫當前頁。
//!
//! 按鍵回撥裡永遠只跑詞級模型；模型的意見在停鍵 80 毫秒後請求、幾十毫秒後到，只換候選窗口裡的整句候選，
//! 使用者翻過頁或動過高亮就不打擾。前文優先用應用裡游標前的文字（DLL 起組句時隨 `ClientMessage::Surrounding` 送來），沒有退回本會話歷史。
//! 節拍由工人迴圈驅動：[`Router::next_tick`] 說下次多久來一次 [`Router::tick`]；DLL 組句期間每 80 ms 的 `Poll` 也順帶 tick。
//! 載入在 [`loader`]，進行態在 [`state`]。

mod loader;
mod state;

use std::path::{Path, PathBuf};
use std::time::Duration;

use qingjian_core::CandidateLayout;
use qingjian_platform::LocalModelConfig;
use qingjian_platform::protocol::SessionId;

use self::loader::Loaded;
pub(crate) use self::loader::ModelLoader;
pub(crate) use self::state::RescoreState;
use super::Router;
use super::composed::{Composed, marked_parts};

/// P2C 優先，同類模型裡使用者目錄優先；沒有 P2C 才回退字級模型。
pub fn find_model(user_dir: Option<&Path>, bundled_root: &Path) -> Option<PathBuf> {
    let candidates = [
        user_dir.map(|dir| dir.join("models/hanzhang-tongbian")),
        user_dir.map(|dir| dir.join("model-p2c")),
        Some(bundled_root.join("data/models/hanzhang-tongbian")),
        user_dir.map(|dir| dir.join("models/hanzhang-zhiwei")),
        user_dir.map(|dir| dir.join("model")),
        Some(bundled_root.join("data/models/hanzhang-zhiwei")),
    ];
    candidates
        .into_iter()
        .flatten()
        .find_map(|dir| qingjian_neural::find_model(&dir))
}

impl Router {
    /// 啟動時：記下模型檔案，按 `[model] enabled` 決定要不要載入。
    pub fn configure_local_model(
        &mut self,
        model_path: Option<PathBuf>,
        config: &LocalModelConfig,
    ) {
        self.model_path = model_path;
        self.applied_model = config.clone();
        if config.enabled {
            self.load_local_model();
        }
    }

    /// 熱載入：`[model]` 變了才過載 / 解除安裝。
    pub(super) fn apply_model_config(&mut self, config: &LocalModelConfig) {
        if *config == self.applied_model {
            return;
        }
        self.applied_model = config.clone();
        if config.enabled {
            self.load_local_model();
        } else {
            self.unload_local_model();
        }
    }

    /// 在後臺執行緒載入模型；沒有模型檔案就什麼都不做。
    fn load_local_model(&mut self) {
        if self.model_loader.is_some() || self.engine.has_sentence_scorer() {
            return;
        }
        let Some(path) = &self.model_path else {
            tracing::info!("没有本地整句模型文件，不重排");
            return;
        };
        self.model_loader = ModelLoader::spawn(path);
    }

    /// 卸掉模型（配置關掉）。
    fn unload_local_model(&mut self) {
        self.model_loader = None;
        self.engine.set_async_sentence_scorer(None);
        self.rescore.stop();
        tracing::info!("本地整句模型已卸载（[model] enabled = false）");
    }

    /// 載入執行緒有結果了就接到 Engine 上；每次按鍵 / tick 順手看一眼，不阻塞。
    pub(super) fn attach_loaded_model(&mut self) {
        let Some(loader) = &self.model_loader else {
            return;
        };
        match loader.poll() {
            Loaded::Pending => {}
            Loaded::Done(result) => {
                match result {
                    Ok(scorer) => {
                        self.engine.set_async_sentence_scorer(Some(scorer));
                        // 模型上線了：日誌裡補一條會話資訊，之後的條目知道重排開著
                        self.engine
                            .log_session(env!("CARGO_PKG_VERSION"), "windows");
                    }
                    Err(error) => tracing::warn!(%error, "本地整句模型加载失败，不重排"),
                }
                self.model_loader = None;
            }
            Loaded::Gone => self.model_loader = None,
        }
    }

    /// 組句結束：什麼都不等了；應用前文也作廢（下一段組句 DLL 會再送）。
    pub(super) fn stop_rescoring(&mut self) {
        self.rescore.stop();
        self.engine.set_rescoring_context(None);
    }

    /// DLL 送來聚焦會話的游標前文：給 Engine 當前文，快取裡按舊前文記的「要打分的」作廢，重新攢一次並重新計時。
    /// 組句已經結束 / 不是聚焦會話的丟掉。
    pub(super) fn set_surrounding(&mut self, session: SessionId, text: String) {
        if self.focused != Some(session) || self.engine.composition().is_empty() {
            return;
        }
        self.engine
            .set_rescoring_context((!text.is_empty()).then_some(text));
        if matches!(self.composed, Some(Composed::Candidates { .. })) {
            // 查一次只為按新前文重新記下要打分的文本，候選順序此刻不變
            let _ = self.engine.query();
            self.schedule_rescoring();
        }
    }

    /// 緩衝變化之後：有整句路徑等著打分就起防抖計時，否則停下。
    pub(super) fn schedule_rescoring(&mut self) {
        if self.engine.rescoring_pending() {
            self.rescore.schedule();
        } else {
            self.rescore.stop();
        }
    }

    /// 工人迴圈下次該多久後來一次 [`Self::tick`]：在等重排就按它的節拍，否則按配置熱載入的一秒。
    pub fn next_tick(&self) -> Duration {
        self.rescore
            .next_deadline()
            .map_or(super::reload::CONFIG_POLL_INTERVAL, |deadline| {
                deadline.min(super::reload::CONFIG_POLL_INTERVAL)
            })
    }

    /// 到點了：接上載入好的模型、推進重排、看一眼配置檔案。工人迴圈超時與 DLL 的 `Poll` 都會調。
    pub fn tick(&mut self) {
        self.attach_loaded_model();
        self.advance_rescoring();
        self.poll_config_reload();
    }

    /// 防抖到點就發請求；在等結果就收一次，收到了重查並重畫當前頁。
    fn advance_rescoring(&mut self) {
        if self.engine.composition().is_empty() || self.translation.is_some() {
            self.rescore.stop();
            return;
        }
        if self.rescore.debounce_elapsed() {
            if self.engine.request_rescoring() {
                self.rescore.start_polling();
            } else {
                self.rescore.stop();
            }
        }
        if !self.rescore.polling() {
            return;
        }
        if !self.engine.poll_rescoring() {
            // 等太久多半是前文變了（上屏後接著打下一段）、結果作廢；真卡住也只是這輪不重排
            if self.rescore.expired() {
                tracing::debug!("等本地整句模型超时，本轮不重排");
                self.rescore.stop();
            }
            return;
        }
        self.rescore.stop();
        if self.highlight >= self.config.page_size || self.navigated {
            return;
        }
        self.requery_rescored();
    }

    /// 分回來了：按重排後的順序重建候選佈局，雲端詞與整句補全留著，重畫當前頁。
    fn requery_rescored(&mut self) {
        let Ok(query) = self.engine.query() else {
            return;
        };
        let Some(Composed::Candidates {
            preedit,
            cursor,
            typed_keys,
            layout,
        }) = self.composed.as_mut()
        else {
            return;
        };
        let cloud = layout.cloud().to_vec();
        let mut rebuilt = CandidateLayout::new(
            query.candidates.items.clone(),
            self.config.page_size,
            self.config.cloud_slots,
        );
        if !cloud.is_empty() {
            rebuilt.set_cloud(cloud);
        }
        *layout = rebuilt;
        (*preedit, *cursor, *typed_keys) = marked_parts(&query);
        let frame = self.self_drawn_frame();
        self.reconcile_candidates(&frame);
        // 前文在結果回來之前換過（Surrounding 晚到）：這次查詢又記下了一批要打分的，再來一輪
        self.schedule_rescoring();
    }
}
