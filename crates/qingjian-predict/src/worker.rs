//! 後臺執行緒：收請求、防抖、查快取、髮網絡請求、回結果。

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use qingjian_core::{Prediction, PredictionRequest};

use crate::prompt::Reply;

use crate::cache::PredictionCache;
use crate::chat_client::ChatClient;
use crate::error::PredictError;

/// 快取條數。
const CACHE_CAPACITY: usize = 64;

pub struct Worker {
    /// 請求入口。主執行緒 drop 掉髮送端後執行緒自然退出。
    requests: Receiver<PredictionRequest>,

    /// 結果出口。
    responses: Sender<Prediction>,

    /// 網路客戶端。
    client: ChatClient,

    /// 防抖視窗。
    debounce: Duration,

    /// 結果快取。
    cache: PredictionCache,
}

/// 快取裡存的是解析後的回覆。
type Cached = Reply;

impl Worker {
    pub fn new(
        requests: Receiver<PredictionRequest>,
        responses: Sender<Prediction>,
        client: ChatClient,
        debounce: Duration,
    ) -> Self {
        Self {
            requests,
            responses,
            client,
            debounce,
            cache: PredictionCache::with_capacity(CACHE_CAPACITY),
        }
    }

    /// 阻塞執行直到傳送端全部關閉。
    pub fn run(mut self) -> Result<(), PredictError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        while let Ok(first) = self.requests.recv() {
            let Some(request) = self.debounce(first) else {
                return Ok(());
            };
            let key = PredictionCache::key(&request);
            if let Some(reply) = self.cache.get(&key) {
                tracing::debug!(sequence = request.sequence, kind = ?request.kind, "联想命中缓存");
                self.reply(request.sequence, reply.clone());
                continue;
            }
            let start = Instant::now();
            match runtime.block_on(self.client.complete(&request)) {
                Ok(reply) => {
                    tracing::info!(
                        sequence = request.sequence,
                        elapsed_ms = start.elapsed().as_millis(),
                        words = reply.words.len(),
                        sentence = reply.sentence.is_some(),
                        "联想完成"
                    );
                    // 空回覆不進快取：模型偶爾什麼都不給（問字尤其），快取住就等於這個問題以後永遠沒答案，
                    // 使用者重打一遍也只會命中快取（2026-09-07 `?mumumu` 就是這麼卡住的）
                    if !reply.is_empty() {
                        self.cache.insert(key, reply.clone());
                    }
                    self.reply(request.sequence, reply);
                }
                Err(error) => {
                    tracing::warn!(sequence = request.sequence, %error, "联想失败");
                }
            }
        }
        Ok(())
    }

    /// 防抖：在視窗內持續收到新請求就一直等，只保留最後一個。傳送端關閉返回 `None`。
    fn debounce(&self, mut latest: PredictionRequest) -> Option<PredictionRequest> {
        loop {
            match self.requests.recv_timeout(self.debounce) {
                Ok(newer) => latest = newer,
                Err(RecvTimeoutError::Timeout) => return Some(latest),
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
    }

    fn reply(&self, sequence: u64, reply: Cached) {
        // 接收端沒了說明 Predictor 已經被 drop，執行緒隨後也會退出
        let _ = self.responses.send(Prediction {
            sequence,
            words: reply.words,
            sentence: reply.sentence,
        });
    }
}
