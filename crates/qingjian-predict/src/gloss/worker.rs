use std::collections::HashSet;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use qingjian_core::{FilledGloss, Language};

use super::prompt;
use crate::chat_client::ChatClient;
use crate::error::PredictError;

/// 收到第一個詞後最多等這麼久再發，攢同一批。
pub const BATCH_WAIT: Duration = Duration::from_millis(1500);

/// 一批最多幾個詞。
pub const BATCH_SIZE: usize = 8;

/// 回覆的 token 上限：8 個詞各兩條短譯詞綽綽有餘。
const MAX_TOKENS: u32 = 600;

/// 後臺執行緒：攢詞、發請求、回釋義。
pub struct GlossWorker {
    /// 請求入口。主執行緒 drop 掉髮送端後執行緒自然退出。
    requests: Receiver<(Language, String)>,

    /// 結果出口。
    responses: Sender<FilledGloss>,

    /// 網路客戶端。
    client: ChatClient,

    /// 本程序內問過的 (語言, 詞)，不再問。
    asked: HashSet<(Language, String)>,
}

impl GlossWorker {
    pub fn new(
        requests: Receiver<(Language, String)>,
        responses: Sender<FilledGloss>,
        client: ChatClient,
    ) -> Self {
        Self {
            requests,
            responses,
            client,
            asked: HashSet::new(),
        }
    }

    /// 阻塞執行直到傳送端全部關閉。
    pub fn run(mut self) -> Result<(), PredictError> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?;
        while let Ok(first) = self.requests.recv() {
            let Some(batch) = self.collect(first) else {
                return Ok(());
            };
            // 一批裡可能混著兩種語言（中途切了學習語言）：按語言各發一次
            let mut languages: Vec<Language> = Vec::new();
            for (language, _) in &batch {
                if !languages.contains(language) {
                    languages.push(*language);
                }
            }
            for language in languages {
                let words: Vec<String> = batch
                    .iter()
                    .filter(|(l, _)| *l == language)
                    .map(|(_, w)| w.clone())
                    .collect();
                self.ask(&runtime, language, words);
            }
        }
        Ok(())
    }

    /// 從第一個詞起攢一批：等到 [`BATCH_WAIT`] 或攢夠 [`BATCH_SIZE`]；問過的跳過。傳送端關閉返回 `None`。
    fn collect(&mut self, first: (Language, String)) -> Option<Vec<(Language, String)>> {
        let deadline = Instant::now() + BATCH_WAIT;
        let mut batch = Vec::with_capacity(BATCH_SIZE);
        self.take(&mut batch, first);
        while batch.len() < BATCH_SIZE {
            let remaining = deadline.saturating_duration_since(Instant::now());
            match self.requests.recv_timeout(remaining) {
                Ok(item) => self.take(&mut batch, item),
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return None,
            }
        }
        Some(batch)
    }

    fn take(&mut self, batch: &mut Vec<(Language, String)>, item: (Language, String)) {
        if self.asked.insert(item.clone()) {
            batch.push(item);
        }
    }

    fn ask(&self, runtime: &tokio::runtime::Runtime, language: Language, words: Vec<String>) {
        if words.is_empty() {
            return;
        }
        let start = Instant::now();
        let system = prompt::system_prompt(language);
        let user = prompt::user_prompt(&words);
        match runtime.block_on(self.client.chat(system, &user, MAX_TOKENS)) {
            Ok(content) => {
                let filled = prompt::parse_reply(&content, language, &words);
                tracing::info!(
                    language = language.code(),
                    asked = words.len(),
                    filled = filled.len(),
                    elapsed_ms = start.elapsed().as_millis(),
                    "释义兜底完成"
                );
                for gloss in filled {
                    // 接收端沒了說明 Filler 已經被 drop，執行緒隨後也會退出
                    if self.responses.send(gloss).is_err() {
                        return;
                    }
                }
            }
            Err(error) => {
                tracing::warn!(language = language.code(), asked = words.len(), %error, "释义兜底失败");
            }
        }
    }
}
