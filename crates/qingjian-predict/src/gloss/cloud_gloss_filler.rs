use std::sync::mpsc::{self, Receiver, Sender};

use qingjian_core::{FilledGloss, GlossFiller, Language};

use super::worker::GlossWorker;
use crate::chat_client::ChatClient;
use crate::config::PredictConfig;
use crate::error::PredictError;

/// 走網路的釋義兜底。構造時起一個後臺執行緒，`request` / `poll` 都只碰通道，不阻塞。
pub struct CloudGlossFiller {
    /// 往後臺執行緒送詞。
    requests: Sender<(Language, String)>,

    /// 從後臺執行緒收釋義。
    responses: Receiver<FilledGloss>,
}

impl CloudGlossFiller {
    /// 與雲聯想共用一份配置（介面、模型、金鑰、超時）；沒有金鑰直接報錯，殼退回 [`qingjian_core::NoGlossFiller`]。
    pub fn new(config: &PredictConfig) -> Result<Self, PredictError> {
        let api_key = config
            .resolve_api_key()
            .ok_or_else(|| PredictError::MissingApiKey(config.api_key_env.clone()))?;
        let client = ChatClient::new(config, api_key);
        let (requests, request_rx) = mpsc::channel();
        let (response_tx, responses) = mpsc::channel();
        let worker = GlossWorker::new(request_rx, response_tx, client);
        std::thread::Builder::new()
            .name("qingjian-gloss".to_owned())
            .spawn(move || {
                if let Err(error) = worker.run() {
                    tracing::error!(%error, "释义兜底线程退出");
                }
            })?;
        tracing::info!("释义兜底已启用");
        Ok(Self {
            requests,
            responses,
        })
    }
}

impl GlossFiller for CloudGlossFiller {
    fn request(&mut self, language: Language, word: &str) {
        if self.requests.send((language, word.to_owned())).is_err() {
            tracing::warn!("释义兜底线程已退出，请求被丢弃");
        }
    }

    fn poll(&mut self) -> Vec<FilledGloss> {
        // Empty 與 Disconnected 都當沒有：執行緒退出時已經記過日誌
        self.responses.try_iter().collect()
    }
}
