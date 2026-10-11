//! 神經重打分：整句轉換的前幾條路徑交給字級模型（[`SentenceScorer`]）再排一次。
//!
//! 打分有兩種接法：同步的（[`Engine::with_sentence_scorer`]，查詢裡當場打，CLI 評測用）和非同步的
//! （[`Engine::with_async_sentence_scorer`]，後臺執行緒；殼裡用）。兩種都經過一張「前文 + 文本 → 神經分」的快取
//! （[`NeuralCache`]）：同步時缺的分當場補進去，非同步時缺的先記下來，殼在使用者停頓後調 [`Engine::request_rescoring`]
//! 一次送去後臺，[`Engine::poll_rescoring`] 收到結果後再查一次，這時全部路徑的分都在快取裡，排序自然換成重排後的。
//! 按鍵回撥永遠不等模型：先按詞級模型出候選，模型的意見晚幾十毫秒到。

mod cache;
mod worker;

#[cfg(test)]
mod tests;

use super::*;

pub(crate) use cache::NeuralCache;
pub(crate) use worker::RescoreWorker;

/// 直接生成整句時的 beam 寬度。5 是 2026-09-24 在凍結集上量的：beam 5 首選 51.9%、前五 73.5%，
/// 再寬只換來零點幾個點，延遲卻線性漲。
const GENERATE_BEAM: usize = 5;

/// 生成長度上限（字）。超過這個長度的輸入使用者早就分段上屏了，放開只會讓最壞情況的延遲沒有上界。
const GENERATE_MAX_CHARS: usize = 32;

/// 生成出來的整句最多取幾條當候選。
const GENERATED_CANDIDATES: usize = 2;

impl Engine {
    /// 接了重打分器（同步或非同步）。
    pub fn has_sentence_scorer(&self) -> bool {
        self.sentence_scorer.is_some()
            || self.rescorer.as_ref().is_some_and(RescoreWorker::is_alive)
    }

    /// 給模型看的前文：殼給了應用裡的游標前文就用它（[`Self::set_rescoring_context`]），
    /// 否則用本會話最近上屏的字元；長度按 `neural_context` 截。
    pub(super) fn rescoring_context(&self) -> String {
        if self.neural_context == 0 {
            return String::new();
        }
        match &self.rescoring_before {
            Some(before) => take_last_chars(before, self.neural_context),
            None => self.history.recent(self.neural_context).to_owned(),
        }
    }

    /// 殼告知應用裡游標前的文本（每次查詢前給；應用給不出就 `None`，退回本會話歷史）。
    pub fn set_rescoring_context(&mut self, before: Option<String>) {
        self.rescoring_before = before;
    }

    /// 把幾條整句路徑按「路徑分 + λ·(神經分 − 靜態分)」重排。快取裡缺分的：同步打分器當場補，非同步的先記下等殼來取；
    /// 有任何一條沒分就不動順序（半截重排比不重排還糟）。
    /// `keys` 是這批路徑共同解釋的那段使用者按鍵（P2C 的條件）。
    /// 只改順序，每條路徑的 `score` 原樣不動（理由見下面的註釋）。
    pub(super) fn rescore_paths(&self, paths: &mut [Conversion], keys: &str) {
        if paths.len() < 2 || !self.has_sentence_scorer() {
            return;
        }
        let context = self.rescoring_context();
        let mut cache = self.neural_cache.borrow_mut();
        cache.ensure_condition(&context, keys);
        let mut missing: Vec<String> = Vec::new();
        for path in paths.iter() {
            if cache.get(&path.text).is_none() && !missing.contains(&path.text) {
                missing.push(path.text.clone());
            }
        }
        if !missing.is_empty() {
            match &self.sentence_scorer {
                Some(scorer) => {
                    let texts: Vec<&str> = missing.iter().map(String::as_str).collect();
                    let scores = scorer.score(&context, keys, &texts);
                    if scores.len() != texts.len() {
                        return;
                    }
                    for (text, score) in texts.iter().zip(scores) {
                        cache.insert(text, score);
                    }
                }
                None => {
                    for text in &missing {
                        cache.want(text);
                    }
                    return;
                }
            }
        }
        // 神經分只決定名次，不寫回 `score`：P2C 打的是 log P(漢字 | 拼音)，被拼音條件住，
        // 比靜態分高十幾二十 nat，抬升還隨句子長短與靜態模型的偏好變化。寫回去會汙染跨讀法的比較
        // （拼寫糾錯的原樣 vs 糾正、混輸的頭段 vs 整段），那兩處的門檻都是按靜態尺度定的：
        // `woxiangxuexirust` 就是這麼被「糾正」成 我想學習如斯 的。
        let lambda = self.neural_weight;
        let rescored = |path: &Conversion| {
            let neural = cache.get(&path.text).expect("filled above");
            path.score + lambda * (neural - path.static_score)
        };
        paths.sort_by(|a, b| {
            rescored(b)
                .partial_cmp(&rescored(a))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        self.last_rescored.set(true);
    }

    /// 不經詞圖，讓模型直接按整段按鍵生成整句，最好的在前。
    ///
    /// 詞圖只會把按鍵讀成拼音，中英混輸（`yongdockerbushuhenfangbian`）與生詞在它那裡沒有路徑，
    /// 出來的只能是把英文段硬讀成拼音的結果（用的哦乘客仍不熟很方便）。這條路不受讀法限制。
    /// 同步打分器當場生成，非同步的先記下、等殼在使用者停頓後取（與重打分同一次請求）。
    pub(super) fn generated_sentences(&self, keys: &str) -> Vec<String> {
        if keys.is_empty() || !self.has_sentence_scorer() {
            return Vec::new();
        }
        let mut cache = self.neural_cache.borrow_mut();
        if let Some(texts) = cache.generated(keys) {
            return texts.iter().take(GENERATED_CANDIDATES).cloned().collect();
        }
        let Some(scorer) = &self.sentence_scorer else {
            cache.want_generation(keys);
            return Vec::new();
        };
        let texts = scorer.generate(keys, GENERATE_BEAM, GENERATE_MAX_CHARS);
        let out = texts.iter().take(GENERATED_CANDIDATES).cloned().collect();
        cache.insert_generated(keys, texts);
        out
    }

    /// 最近一次查詢裡有整句路徑還沒拿到神經分、或有整段還沒生成：殼該在使用者停頓後調 [`Self::request_rescoring`]。
    pub fn rescoring_pending(&self) -> bool {
        self.rescorer.is_some() && {
            let cache = self.neural_cache.borrow();
            cache.has_wanted() || cache.wanted_generation().is_some()
        }
    }

    /// 把攢著的文本送去後臺打分。沒接非同步打分器或沒什麼要打的返回 `false`。
    pub fn request_rescoring(&mut self) -> bool {
        let Some(worker) = &self.rescorer else {
            return false;
        };
        let mut cache = self.neural_cache.borrow_mut();
        let wanted = cache.take_wanted();
        let generate = cache.take_wanted_generation();
        if wanted.is_empty() && generate.is_none() {
            return false;
        }
        tracing::debug!(
            texts = wanted.len(),
            generate = generate.is_some(),
            "神经请求"
        );
        worker.submit(
            cache.context().to_owned(),
            cache.keys().to_owned(),
            wanted,
            generate,
        );
        true
    }

    /// 收後臺打好的分。有新分進了快取返回 `true`，殼該重新 [`Self::query`] 一次；前文已經變了的結果丟掉。
    pub fn poll_rescoring(&mut self) -> bool {
        let Some(worker) = &self.rescorer else {
            return false;
        };
        let mut updated = false;
        while let Some(scored) = worker.poll() {
            let mut cache = self.neural_cache.borrow_mut();
            // 生成的結果自帶按鍵、與前文無關，不跟著打分那邊的條件一起作廢
            if let Some((keys, texts)) = scored.generated {
                cache.insert_generated(&keys, texts);
                updated = true;
            }
            if scored.context != cache.context()
                || scored.keys != cache.keys()
                || scored.scores.len() != scored.texts.len()
            {
                continue;
            }
            for (text, score) in scored.texts.iter().zip(scored.scores) {
                cache.insert(text, score);
            }
            updated = true;
        }
        updated
    }
}
