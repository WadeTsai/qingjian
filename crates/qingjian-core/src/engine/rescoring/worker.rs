use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;

use crate::sentence::SentenceScorer;

use super::{GENERATE_BEAM, GENERATE_MAX_CHARS};

/// 一次後臺任務：兩個條件與一批要打分的文本，外加可選的「直接按這段按鍵生成整句」。
/// 兩件事合成一條任務是因為它們都在使用者停頓後一起發出，而排隊的任務只算最新一條。
struct Job {
    context: String,
    keys: String,
    texts: Vec<String>,

    /// 要生成整句的那段按鍵（整段作用域）；`None` 就只打分。
    generate: Option<String>,
}

/// 後臺算好的結果，與任務一一對應。
pub(crate) struct Scored {
    pub context: String,
    pub keys: String,
    pub texts: Vec<String>,
    pub scores: Vec<f64>,

    /// 生成用的按鍵與生成出來的整句，對應任務裡的 `generate`。
    pub generated: Option<(String, Vec<String>)>,
}

/// 後臺打分執行緒：模型前向要幾十毫秒，不能放在按鍵回撥裡。
/// 任務排隊時只算最新的一條（舊的對應已經過去的輸入狀態）；執行緒隨本結構一起結束。
pub(crate) struct RescoreWorker {
    jobs: Sender<Job>,
    results: Receiver<Scored>,
    handle: Option<JoinHandle<()>>,
}

impl RescoreWorker {
    pub fn spawn(scorer: Box<dyn SentenceScorer>) -> Self {
        let (jobs, job_rx) = channel::<Job>();
        let (result_tx, results) = channel::<Scored>();
        let handle = std::thread::Builder::new()
            .name("qingjian-rescore".to_owned())
            .spawn(move || {
                while let Ok(mut job) = job_rx.recv() {
                    // 攢了好幾條只算最後一條
                    while let Ok(newer) = job_rx.try_recv() {
                        job = newer;
                    }
                    let texts: Vec<&str> = job.texts.iter().map(String::as_str).collect();
                    let started = std::time::Instant::now();
                    let scores = scorer.score(&job.context, &job.keys, &texts);
                    tracing::debug!(
                        texts = texts.len(),
                        context_chars = job.context.chars().count(),
                        keys = job.keys.len(),
                        ms = started.elapsed().as_millis(),
                        "神经重打分完成"
                    );
                    let generated = job.generate.map(|keys| {
                        let started = std::time::Instant::now();
                        let texts = scorer.generate(&keys, GENERATE_BEAM, GENERATE_MAX_CHARS);
                        tracing::debug!(
                            keys = keys.len(),
                            got = texts.len(),
                            ms = started.elapsed().as_millis(),
                            "整句生成完成"
                        );
                        (keys, texts)
                    });
                    let done = Scored {
                        context: job.context,
                        keys: job.keys,
                        texts: job.texts,
                        scores,
                        generated,
                    };
                    if result_tx.send(done).is_err() {
                        break;
                    }
                }
            })
            .ok();
        if handle.is_none() {
            tracing::warn!("起不了神经重打分线程，本次不用模型");
        }
        Self {
            jobs,
            results,
            handle,
        }
    }

    pub fn is_alive(&self) -> bool {
        self.handle.is_some()
    }

    pub fn submit(
        &self,
        context: String,
        keys: String,
        texts: Vec<String>,
        generate: Option<String>,
    ) {
        if self
            .jobs
            .send(Job {
                context,
                keys,
                texts,
                generate,
            })
            .is_err()
        {
            tracing::warn!("神经重打分线程已退出");
        }
    }

    /// 取一條打好的分；沒有就 `None`。
    pub fn poll(&self) -> Option<Scored> {
        self.results.try_recv().ok()
    }
}

impl Drop for RescoreWorker {
    fn drop(&mut self) {
        // 關掉任務通道執行緒就會退出；不等它（模型可能正算到一半）
        let _ = self.handle.take();
    }
}
