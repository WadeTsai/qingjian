//! 本地整句模型：後臺載入、停頓後請求重排、結果到了重畫當前頁。
//!
//! 按鍵回撥裡永遠只跑詞級模型；模型的意見在停鍵 80 毫秒後請求、二三十毫秒後到，只換候選窗口裡的整句候選，
//! 使用者翻過頁或動過高亮就不打擾。前文優先用應用裡游標前的文字（`refresh` 每次查詢前給 Engine），應用給不出退回本會話歷史。

use std::sync::mpsc::{TryRecvError, channel};

use qingjian_core::sentence::SentenceScorer;
use qingjian_neural::{CharScorer, NeuralError, P2cScorer};

mod rescore_monitor;

pub(super) use rescore_monitor::RescoreMonitor;

use super::*;

impl Host {
    /// 在後臺執行緒載入模型並預熱（第一次前向要編譯 Metal 核心，幾百毫秒），載入完由 [`Self::attach_loaded_model`] 接上。
    /// 沒有模型檔案就什麼都不做。
    pub(super) fn load_local_model(&mut self) {
        if self.model_loader.is_some() || self.engine.has_sentence_scorer() {
            return;
        }
        // P2C 優先：凍結集 8322 句上比字級模型的重排高 0.69 個點、延遲還低（docs/notes/model-eval.md）
        let (path, p2c) = match paths::p2c_model_path() {
            Some(path) => (path, true),
            None => match paths::model_path() {
                Some(path) => (path, false),
                None => {
                    tracing::info!("没有本地模型文件，不重排");
                    return;
                }
            },
        };
        let (tx, rx) = channel::<Result<Box<dyn SentenceScorer>, NeuralError>>();
        let spawned = std::thread::Builder::new()
            .name("qingjian-model-load".to_owned())
            .spawn(move || {
                let started = std::time::Instant::now();
                // 預熱要走各自真正的前向：第一次前向要編譯 Metal 核心，幾百毫秒
                let loaded = CharScorer::load(&path).and_then(|scorer| {
                    if p2c {
                        let scorer = P2cScorer::new(scorer)
                            .ok_or(NeuralError::Corrupt("含章·通变模型缺少 <sep> 分隔符"))?;
                        scorer.0.score_p2c("ni", &["你"])?;
                        Ok(Box::new(scorer) as Box<dyn SentenceScorer>)
                    } else {
                        scorer.score("", &["的"])?;
                        Ok(Box::new(scorer) as Box<dyn SentenceScorer>)
                    }
                });
                if loaded.is_ok() {
                    tracing::info!(
                        path = %path.display(),
                        kind = if p2c { "含章·通变" } else { "含章·知微" },
                        total_ms = started.elapsed().as_millis(),
                        "本地模型已加载并预热"
                    );
                }
                let _ = tx.send(loaded);
            });
        match spawned {
            Ok(_) => {
                self.model_loader = Some(rx);
                self.rescore.watch_loading();
            }
            Err(error) => tracing::warn!(%error, "起不了模型加载线程，本地整句模型不用"),
        }
    }

    /// 載入執行緒有結果了就接到 Engine 上；每次查詢和載入定時器都會看一眼，不阻塞。
    pub fn attach_loaded_model(&mut self) {
        let Some(rx) = &self.model_loader else {
            self.rescore.stop_watching();
            return;
        };
        match rx.try_recv() {
            Ok(Ok(scorer)) => {
                self.engine.set_async_sentence_scorer(Some(scorer));
                self.model_loader = None;
                self.rescore.stop_watching();
                // 模型上線了：日誌裡補一條會話資訊，之後的條目知道重排開著
                let version = self.version.clone();
                self.engine.log_session(&version, "macos");
                self.rescore_current_round();
            }
            Ok(Err(error)) => {
                tracing::warn!(%error, "本地模型加载失败，不重排");
                self.model_loader = None;
                self.rescore.stop_watching();
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                self.model_loader = None;
                self.rescore.stop_watching();
            }
        }
    }

    /// 模型接上時使用者正在組句：這一輪的查詢從沒見過打分器，不補查一次就永遠錯過重排。
    /// 補查只為攢下整句路徑、起防抖，不動畫面；使用者已翻頁或動過高亮就不打擾。
    fn rescore_current_round(&mut self) {
        if self.engine.composition().is_empty()
            || self.translation.is_some()
            || self.session.page != 0
            || self.session.navigated
        {
            return;
        }
        if self.engine.query().is_ok() {
            tracing::info!("模型接上时正在组句，补一轮重排");
            self.schedule_rescoring();
        }
    }

    /// 模型還在後臺載入。
    pub fn model_loading(&self) -> bool {
        self.model_loader.is_some()
    }

    /// 卸掉模型（配置關掉）。
    pub(super) fn unload_local_model(&mut self) {
        self.model_loader = None;
        self.engine.set_async_sentence_scorer(None);
        self.rescore.stop_watching();
        self.rescore.stop();
    }

    /// 每次查詢之後：有整句路徑等著打分就起防抖計時。
    pub fn schedule_rescoring(&mut self) {
        if self.engine.rescoring_pending() {
            self.rescore.schedule();
        }
    }

    /// 防抖到點：把攢著的整句路徑送去後臺，開始輪詢。
    pub fn start_rescoring(&mut self) {
        if self.engine.composition().is_empty() {
            self.rescore.stop();
            return;
        }
        if self.engine.request_rescoring() {
            self.rescore.start_polling();
        }
    }

    /// 輪詢到點：分回來了就重查一次、重畫當前頁；使用者已翻頁或動過高亮就只留著分不動畫面。
    pub fn poll_rescoring(&mut self) {
        if self.engine.composition().is_empty() || self.translation.is_some() {
            self.rescore.stop();
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
        if self.session.page != 0 || self.session.navigated {
            return;
        }
        let Ok(mut query) = self.engine.query() else {
            return;
        };
        self.engine.annotate(&mut query.candidates);
        let preedit = Preedit::from_marked(&query.marked_segments(), query.segments_cursor());
        let cloud = self.session.layout.cloud().to_vec();
        self.reset_session(preedit, query.candidates.items);
        if !cloud.is_empty() {
            self.session.layout.set_cloud(cloud);
        }
        self.render();
    }
}
