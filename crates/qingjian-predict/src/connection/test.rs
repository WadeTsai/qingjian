//! 雲服務連通性測試：偏好設定「雲服務」頁的「測試連線」按鈕用。
//! 起一個執行緒用當前配置發一條最小的聊天請求，結果經通道回來；呼叫方在主執行緒輪詢 [`ConnectionTest::poll`]。

use std::sync::mpsc;
use std::time::Instant;

use super::report::ConnectionReport;
use crate::chat_client::ChatClient;
use crate::config::PredictConfig;
use crate::error::PredictError;

/// 系統提示：告訴模型這只是探活。
const SYSTEM_PROMPT: &str = "你是输入法的连通性测试。";

/// 使用者訊息：要一個固定的 JSON，回覆越短越好。
const USER_PROMPT: &str = "回复 JSON 对象 {\"ok\": true}，不要别的内容。";

/// 回覆 token 上限：只要幾個字。
const MAX_TOKENS: u32 = 32;

/// 一次進行中的連通性測試。
pub struct ConnectionTest {
    /// 測試執行緒送回的結果；執行緒只發一次。
    result: mpsc::Receiver<Result<ConnectionReport, PredictError>>,
}

impl ConnectionTest {
    /// 按配置發起測試。金鑰缺失、執行緒起不來這兩種情況當場報錯，其餘錯誤從 [`poll`](Self::poll) 回來。
    pub fn start(config: &PredictConfig) -> Result<Self, PredictError> {
        let api_key = config
            .resolve_api_key()
            .ok_or_else(|| PredictError::MissingApiKey(config.api_key_env.clone()))?;
        let client = ChatClient::new(config, api_key);
        let model = config.model.clone();
        let (sender, result) = mpsc::channel();
        std::thread::Builder::new()
            .name("qingjian-connection-test".to_owned())
            .spawn(move || {
                let outcome = run(&client, model);
                match &outcome {
                    Ok(report) => tracing::info!(
                        model = %report.model,
                        elapsed_ms = report.elapsed.as_millis() as u64,
                        reply = %report.reply,
                        "云服务连通性测试成功"
                    ),
                    Err(error) => tracing::warn!(error = ?error, "云服务连通性测试失败"),
                }
                // 收的一方不在了（視窗關了、又點了一次）就算了
                let _ = sender.send(outcome);
            })?;
        Ok(Self { result })
    }

    /// 結果到了就取走；沒到返回 `None`。取走之後再調永遠是 `None`。
    pub fn poll(&self) -> Option<Result<ConnectionReport, PredictError>> {
        self.result.try_recv().ok()
    }
}

/// 在測試執行緒裡跑：建一個單執行緒執行時，發一條請求，計時。
fn run(client: &ChatClient, model: String) -> Result<ConnectionReport, PredictError> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    let started = Instant::now();
    let reply = runtime.block_on(client.chat(SYSTEM_PROMPT, USER_PROMPT, MAX_TOKENS))?;
    Ok(ConnectionReport {
        model,
        elapsed: started.elapsed(),
        reply: reply.trim().to_owned(),
    })
}
