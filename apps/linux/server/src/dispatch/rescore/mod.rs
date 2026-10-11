//! 本地整句模型（與 Windows Server 的 `dispatch/rescore` 對齊）：後臺載入、停鍵後請求重排、結果到了重建當前佈局。
//!
//! 按鍵回撥裡永遠只跑詞級模型；模型的意見在停鍵 80 毫秒後請求、幾十毫秒後到，只換候選裡的整句候選，
//! 使用者翻過頁或動過高亮就不打擾。前文用本會話最近上屏的字（首版不讀應用裡游標前的文字）。
//! 節拍由主迴圈驅動：[`Router::next_tick`] 說下次多久來一次 [`Router::tick`]；外掛組句期間每 80 毫秒的 `Poll` 也順帶 tick。
//! 模型在後臺接上時使用者正在組句，這一輪也補一次重排。載入在 [`loader`]，進行態在 [`state`]。

mod loader;
mod state;

use std::path::{Path, PathBuf};
use std::time::Duration;

use qingjian_core::CandidateLayout;
use qingjian_core::sentence::SentenceScorer;
use qingjian_platform::LocalModelConfig;

use self::loader::Loaded;
pub(crate) use self::loader::ModelLoader;
pub(crate) use self::state::RescoreState;
use super::Router;
use super::composed::Composed;

/// 空閒時主迴圈多久醒一次：到點落盤學習資料。
const IDLE_TICK: Duration = Duration::from_secs(1);

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
    /// 啟動時：記下模型檔案，按 `[model] enabled` 決定要不要載入。Linux 沒有配置熱載入，關了要重啟服務。
    pub fn configure_local_model(
        &mut self,
        model_path: Option<PathBuf>,
        config: &LocalModelConfig,
    ) {
        self.model_path = model_path;
        if config.enabled {
            self.load_local_model();
        } else {
            tracing::info!("本地整句模型已关（[model] enabled = false）");
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

    /// 載入執行緒有結果了就接到 Engine 上；每次按鍵 / tick 順手看一眼，不阻塞。
    pub(super) fn attach_loaded_model(&mut self) {
        let Some(loader) = &self.model_loader else {
            return;
        };
        match loader.poll() {
            Loaded::Pending => {}
            Loaded::Done(result) => {
                match result {
                    Ok(scorer) => self.attach_sentence_scorer(scorer),
                    Err(error) => tracing::warn!(%error, "本地整句模型加载失败，不重排"),
                }
                self.model_loader = None;
            }
            Loaded::Gone => self.model_loader = None,
        }
    }

    /// 打分器接到 Engine 上（後臺載入完成，或測試直接注入）：日誌補一條會話資訊，正在組句就補這一輪的重排。
    pub fn attach_sentence_scorer(&mut self, scorer: Box<dyn SentenceScorer>) {
        self.engine.set_async_sentence_scorer(Some(scorer));
        // 模型上線了：日誌裡補一條會話資訊，之後的條目知道重排開著
        self.engine.log_session(env!("CARGO_PKG_VERSION"), "linux");
        self.rescore_current_round();
    }

    /// 模型接上時使用者正在組句：這一輪的查詢從沒見過打分器，不補查一次就永遠錯過重排。
    /// 補查只為攢下整句路徑、起防抖，不動畫面；使用者已翻頁或動過高亮就不打擾。
    fn rescore_current_round(&mut self) {
        if !matches!(self.composed, Some(Composed::Candidates { .. })) || !self.on_first_page() {
            return;
        }
        if self.engine.query().is_ok() {
            tracing::info!("模型接上时正在组句，补一轮重排");
            self.schedule_rescoring();
        }
    }

    /// 組句結束 / 換會話：什麼都不等了。
    pub(super) fn stop_rescoring(&mut self) {
        self.rescore.stop();
    }

    /// 緩衝變化之後：有整句路徑等著打分就起防抖計時，否則停下。
    pub(super) fn schedule_rescoring(&mut self) {
        if self.engine.rescoring_pending() {
            self.rescore.schedule();
        } else {
            self.rescore.stop();
        }
    }

    /// 主迴圈下次該多久後來一次 [`Router::tick`]：在等重排就按它的節拍，否則按落盤學習的一秒。
    pub fn next_tick(&self) -> Duration {
        self.rescore
            .next_deadline()
            .map_or(IDLE_TICK, |deadline| deadline.min(IDLE_TICK))
    }

    /// 防抖到點就發請求；在等結果就收一次，收到了重查並重建當前佈局。
    pub(super) fn advance_rescoring(&mut self) {
        if self.engine.composition().is_empty() {
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
        if self.on_first_page() {
            self.requery_rescored();
        }
    }

    /// 使用者還看著第一頁、沒動過高亮：模型的結果才換正在看的這頁。
    fn on_first_page(&self) -> bool {
        self.highlight < self.config.page_size && !self.navigated
    }

    /// 分回來了：按重排後的順序重建候選佈局，下一次 `Poll` 回的幀就是新順序。
    fn requery_rescored(&mut self) {
        let Ok(query) = self.engine.query() else {
            return;
        };
        let Some(Composed::Candidates {
            preedit,
            cursor,
            layout,
        }) = self.composed.as_mut()
        else {
            return;
        };
        *layout = CandidateLayout::new(
            query.candidates.items.clone(),
            self.config.page_size,
            self.config.cloud_slots,
        );
        *preedit = query.marked_segments().iter().map(Into::into).collect();
        *cursor = query.marked_cursor();
        // 這次查詢可能又記下了一批要打分的（快取按前文記，前文沒變時不會），再來一輪
        self.schedule_rescoring();
    }
}

#[cfg(test)]
mod tests {
    use super::find_model;

    #[test]
    fn p2c_precedes_old_model_and_user_precedes_bundled() {
        let root =
            std::env::temp_dir().join(format!("qingjian-model-choice-{}", std::process::id()));
        let user = root.join("user");
        let bundled = root.join("bundled");
        let _ = std::fs::remove_dir_all(&root);

        let old = bundled.join("data/models/hanzhang-zhiwei/model.qjm");
        std::fs::create_dir_all(old.parent().unwrap()).unwrap();
        std::fs::write(&old, b"").unwrap();
        assert_eq!(find_model(Some(&user), &bundled), Some(old));

        let user_old = user.join("models/hanzhang-zhiwei/model.qjm");
        std::fs::create_dir_all(user_old.parent().unwrap()).unwrap();
        std::fs::write(&user_old, b"").unwrap();
        assert_eq!(find_model(Some(&user), &bundled), Some(user_old));

        let bundled_p2c = bundled.join("data/models/hanzhang-tongbian/model.qjm");
        std::fs::create_dir_all(bundled_p2c.parent().unwrap()).unwrap();
        std::fs::write(&bundled_p2c, b"").unwrap();
        assert_eq!(find_model(Some(&user), &bundled), Some(bundled_p2c));

        let user_p2c = user.join("models/hanzhang-tongbian/model.qjm");
        std::fs::create_dir_all(user_p2c.parent().unwrap()).unwrap();
        std::fs::write(&user_p2c, b"").unwrap();
        assert_eq!(find_model(Some(&user), &bundled), Some(user_p2c.clone()));
        let legacy_p2c = user.join("model-p2c/model.qjm");
        std::fs::create_dir_all(legacy_p2c.parent().unwrap()).unwrap();
        std::fs::write(&legacy_p2c, b"").unwrap();
        assert_eq!(find_model(Some(&user), &bundled), Some(user_p2c.clone()));
        std::fs::remove_file(&user_p2c).unwrap();
        assert_eq!(find_model(Some(&user), &bundled), Some(legacy_p2c));
        std::fs::remove_dir_all(&root).unwrap();
    }
}
