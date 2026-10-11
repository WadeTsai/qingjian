use std::collections::HashMap;

/// 神經分快取：一組條件（前文 + 使用者按鍵）下各整句文本的神經分。
///
/// 一次查詢裡整句轉換會跑好幾遍（糾錯變體、中英混輸比分……），同一段前文下同一條文本只該問模型一次；
/// 非同步打分時查詢先把「還沒分的文本」攢在 `wanted` 裡，殼在停頓後一次送去後臺，結果回來按文本填進來，
/// 再查一次就都在快取裡了。前文一變整張表作廢。
#[derive(Debug, Default)]
pub(crate) struct NeuralCache {
    /// 這些分數對應的前文。
    context: String,

    /// 這些分數對應的使用者按鍵（P2C 的條件；字級模型用不到，但兩者一起構成快取身份）。
    keys: String,

    /// 文本 → 神經分。
    scores: HashMap<String, f64>,

    /// 還沒有分、等著送去後臺的文本（不重複）。
    wanted: Vec<String>,

    /// 模型不經詞圖、直接按按鍵生成的整句，連同生成時用的那段按鍵。
    ///
    /// 這段按鍵與打分用的 `keys` 不是一回事：打分的條件是某一批路徑覆蓋的那段（英文尾巴那條只覆蓋頭段），
    /// 生成的條件永遠是整段作用域。所以自帶一份鍵，不跟著 [`Self::ensure_condition`] 作廢。
    /// 前文不參與：P2C 只認按鍵。
    generated: Option<(String, Vec<String>)>,

    /// 還沒生成、等著送去後臺的那段按鍵。
    generation_wanted: Option<String>,
}

/// 快取最多留多少條文本：前文不變時一次組句裡的候選也就幾十條，超過說明有別的東西在刷。
const MAX_ENTRIES: usize = 1024;

impl NeuralCache {
    /// 換條件：前文或按鍵任一變了就整張清掉。
    pub fn ensure_condition(&mut self, context: &str, keys: &str) {
        if self.context != context || self.keys != keys {
            self.context.clear();
            self.context.push_str(context);
            self.keys.clear();
            self.keys.push_str(keys);
            self.scores.clear();
            self.wanted.clear();
        }
        if self.scores.len() > MAX_ENTRIES {
            self.scores.clear();
        }
    }

    pub fn context(&self) -> &str {
        &self.context
    }

    pub fn keys(&self) -> &str {
        &self.keys
    }

    pub fn get(&self, text: &str) -> Option<f64> {
        self.scores.get(text).copied()
    }

    pub fn insert(&mut self, text: &str, score: f64) {
        self.scores.insert(text.to_owned(), score);
    }

    /// 記下一條要分的文本；已經有分或已經在等的不重複記。
    pub fn want(&mut self, text: &str) {
        if !self.scores.contains_key(text) && !self.wanted.iter().any(|w| w == text) {
            self.wanted.push(text.to_owned());
        }
    }

    pub fn has_wanted(&self) -> bool {
        !self.wanted.is_empty()
    }

    /// 取走等著送去後臺的文本。
    pub fn take_wanted(&mut self) -> Vec<String> {
        std::mem::take(&mut self.wanted)
    }

    /// 這段按鍵上生成好的整句；還沒生成過返回 `None`。
    pub fn generated(&self, keys: &str) -> Option<&[String]> {
        self.generated
            .as_ref()
            .filter(|(cached, _)| cached == keys)
            .map(|(_, texts)| texts.as_slice())
    }

    pub fn insert_generated(&mut self, keys: &str, texts: Vec<String>) {
        self.generation_wanted = None;
        self.generated = Some((keys.to_owned(), texts));
    }

    /// 記下要給這段按鍵生成；已經生成過的不重複記。
    pub fn want_generation(&mut self, keys: &str) {
        if self.generated(keys).is_none() {
            self.generation_wanted = Some(keys.to_owned());
        }
    }

    pub fn wanted_generation(&self) -> Option<&str> {
        self.generation_wanted.as_deref()
    }

    /// 取走等著送去後臺生成的按鍵。
    pub fn take_wanted_generation(&mut self) -> Option<String> {
        self.generation_wanted.take()
    }
}
