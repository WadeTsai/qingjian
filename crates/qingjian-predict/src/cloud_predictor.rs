use std::sync::mpsc::{self, Receiver, Sender};
use std::time::Duration;

use qingjian_core::{Prediction, PredictionPolicy, PredictionRequest, Predictor};

use crate::chat_client::ChatClient;
use crate::config::PredictConfig;
use crate::error::PredictError;
use crate::worker::Worker;

/// 走網路的聯想器。構造時起一個後臺執行緒，`submit` / `poll` 都只碰通道，不阻塞。
pub struct CloudPredictor {
    /// 觀察視窗、條數上限與開關，來自配置。
    policy: PredictionPolicy,

    /// 往後臺執行緒發請求。
    requests: Sender<PredictionRequest>,

    /// 從後臺執行緒收結果。
    responses: Receiver<Prediction>,
}

impl CloudPredictor {
    /// 沒有金鑰直接報錯，讓殼退回 [`qingjian_core::NoPredictor`] 並記日誌。
    pub fn new(config: &PredictConfig) -> Result<Self, PredictError> {
        let api_key = config
            .resolve_api_key()
            .ok_or_else(|| PredictError::MissingApiKey(config.api_key_env.clone()))?;
        let client = ChatClient::new(config, api_key);
        let (requests, request_rx) = mpsc::channel();
        let (response_tx, responses) = mpsc::channel();
        let worker = Worker::new(
            request_rx,
            response_tx,
            client,
            Duration::from_millis(config.debounce_ms),
        );
        std::thread::Builder::new()
            .name("qingjian-predict".to_owned())
            .spawn(move || {
                if let Err(error) = worker.run() {
                    tracing::error!(%error, "联想线程退出");
                }
            })?;
        tracing::info!(
            base_url = %config.base_url,
            model = %config.model,
            lookback = config.lookback,
            lookahead = config.lookahead,
            "云联想已启用"
        );
        Ok(Self {
            policy: config.policy(),
            requests,
            responses,
        })
    }
}

impl Predictor for CloudPredictor {
    fn policy(&self) -> PredictionPolicy {
        self.policy
    }

    fn submit(&mut self, request: PredictionRequest) {
        if self.requests.send(request).is_err() {
            tracing::warn!("联想线程已退出，请求被丢弃");
        }
    }

    fn poll(&mut self) -> Option<Prediction> {
        // Empty 與 Disconnected 都當沒有：執行緒退出時已經記過日誌
        self.responses.try_recv().ok()
    }
}
