//! 模型的後臺載入：載入並預熱要幾百毫秒到幾秒，不能擋住按鍵；載入完由 Router 在下一次訊息時接上。

use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, TryRecvError, channel};

use qingjian_core::sentence::SentenceScorer;
use qingjian_neural::{CharScorer, NeuralError, P2cScorer};

/// 一次進行中的載入。
pub(crate) struct ModelLoader {
    result: Receiver<Result<Box<dyn SentenceScorer>, NeuralError>>,
}

/// 一次載入的結局。
pub(super) enum Loaded {
    /// 還在載入。
    Pending,

    /// 載入好了（或失敗了）。
    Done(Result<Box<dyn SentenceScorer>, NeuralError>),

    /// 載入執行緒沒了（起不來 / panic）。
    Gone,
}

impl ModelLoader {
    /// 起執行緒載入 `path`（`.qjm` 或三件套目錄）並預熱一次；執行緒起不來返回 `None`。
    pub(super) fn spawn(path: &Path) -> Option<Self> {
        let path: PathBuf = path.to_path_buf();
        let (tx, result) = channel();
        let spawned = std::thread::Builder::new()
            .name("qingjian-model-load".to_owned())
            .spawn(move || {
                let started = std::time::Instant::now();
                let loaded = CharScorer::load(&path).and_then(|scorer| {
                    if scorer.vocab().sep().is_some() {
                        let p2c = P2cScorer::new(scorer).expect("P2C vocab checked above");
                        p2c.0.score_p2c("ni", &["你"])?;
                        Ok((Box::new(p2c) as Box<dyn SentenceScorer>, "含章·通变"))
                    } else {
                        scorer.score("", &["的"])?;
                        Ok((Box::new(scorer) as Box<dyn SentenceScorer>, "含章·知微"))
                    }
                });
                if let Ok((_, kind)) = &loaded {
                    tracing::info!(
                        path = %path.display(),
                        kind,
                        total_ms = started.elapsed().as_millis(),
                        "本地整句模型已加载并预热"
                    );
                }
                let _ = tx.send(loaded.map(|(scorer, _)| scorer));
            });
        match spawned {
            Ok(_) => Some(Self { result }),
            Err(error) => {
                tracing::warn!(%error, "起不了模型加载线程，本地整句模型不用");
                None
            }
        }
    }

    /// 看一眼有沒有結果，不阻塞。
    pub(super) fn poll(&self) -> Loaded {
        match self.result.try_recv() {
            Ok(result) => Loaded::Done(result),
            Err(TryRecvError::Empty) => Loaded::Pending,
            Err(TryRecvError::Disconnected) => Loaded::Gone,
        }
    }
}
