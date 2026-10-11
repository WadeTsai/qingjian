//! 分批併發請求的通用驅動：一批詞 → 提示詞 → 模型 → 解析 → 落盤；沒拿到的詞再問，最多幾輪。
//! 釋義生成與拼音標註共用，差別只在提示詞與解析函式。

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use tokio::sync::Semaphore;
use tokio::task::JoinSet;

use crate::client::LlmClient;
use crate::store::{Keyed, Store};

/// 一批詞最多問幾輪：第一輪整批，之後只問上一輪沒答或答壞的詞。
const MAX_ROUNDS: usize = 3;

/// 一種任務：系統提示詞、使用者訊息怎麼拼、回覆怎麼解析。
pub trait Task: Send + Sync + 'static {
    /// 一行結果的型別。
    type Entry: Keyed + Send + Sync + 'static;

    fn system_prompt(&self) -> &str;

    fn user_prompt(&self, words: &[String]) -> String;

    /// 只收請求過的詞，壞的丟掉。
    fn parse_reply(
        &self,
        content: &str,
        words: &[String],
    ) -> Result<Vec<Self::Entry>, serde_json::Error>;
}

/// 併發跑完 `pending` 裡的詞。`batch` 每請求幾個詞，`concurrency` 同時幾個請求。
pub async fn run<T: Task>(
    task: Arc<T>,
    client: Arc<LlmClient>,
    store: Arc<Store<T::Entry>>,
    pending: Vec<String>,
    batch: usize,
    concurrency: usize,
) {
    let semaphore = Arc::new(Semaphore::new(concurrency.max(1)));
    let done = Arc::new(AtomicUsize::new(0));
    let total = pending.len();
    let mut tasks = JoinSet::new();
    for chunk in pending.chunks(batch.max(1)) {
        let words: Vec<String> = chunk.to_vec();
        let (task, client, store, semaphore, done) = (
            task.clone(),
            client.clone(),
            store.clone(),
            semaphore.clone(),
            done.clone(),
        );
        tasks.spawn(async move {
            let _permit = semaphore.acquire().await;
            let got = process_batch(&*task, &client, &store, words).await;
            let finished = done.fetch_add(got, Ordering::Relaxed) + got;
            tracing::info!(done = finished, total, "进度");
        });
    }
    while let Some(result) = tasks.join_next().await {
        if let Err(error) = result {
            tracing::warn!(%error, "任务异常");
        }
    }
    tracing::info!(done = done.load(Ordering::Relaxed), total, "结束");
}

/// 處理一批詞：請求、解析、落盤；沒拿到的詞再問，最多幾輪。返回拿到的條目數。
async fn process_batch<T: Task>(
    task: &T,
    client: &LlmClient,
    store: &Store<T::Entry>,
    mut batch: Vec<String>,
) -> usize {
    let mut got = 0;
    for round in 1..=MAX_ROUNDS {
        if batch.is_empty() {
            break;
        }
        let user = task.user_prompt(&batch);
        let content = match client.complete(task.system_prompt(), &user).await {
            Ok(content) => content,
            Err(error) => {
                tracing::warn!(round, words = batch.len(), %error, "请求失败");
                tokio::time::sleep(Duration::from_secs(2 * round as u64)).await;
                continue;
            }
        };
        let entries = match task.parse_reply(&content, &batch) {
            Ok(entries) => entries,
            Err(error) => {
                tracing::warn!(round, %error, "回复不是合法 JSON");
                continue;
            }
        };
        if let Err(error) = store.append(&entries) {
            tracing::error!(%error, "写结果失败");
            return got;
        }
        got += entries.len();
        batch.retain(|w| !entries.iter().any(|e| e.key() == w));
        if !batch.is_empty() {
            tracing::debug!(round, missing = ?batch, "有词没答，再问");
        }
    }
    if !batch.is_empty() {
        tracing::warn!(missing = ?batch, "放弃");
    }
    got
}
