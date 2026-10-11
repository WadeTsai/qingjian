/// 整句路徑的第二打分來源：給 Viterbi 出的前幾條路徑再打一次分，與路徑本身的得分對數線性插值。
/// Core 只認這個 trait，實現在 `qingjian-neural`。
pub trait SentenceScorer: Send {
    /// 每條 `texts` 的 log 機率，與 `texts` 一一對應。算不了（模型出錯）返回空 Vec，呼叫方就當沒有這個打分。
    ///
    /// 兩個條件都給，實現按自己訓練時的條件挑用、忽略另一個：
    /// `context` 是游標前文（字級模型用），`keys` 是這批路徑共同解釋的那段使用者按鍵（P2C 用）。
    fn score(&self, context: &str, keys: &str, texts: &[&str]) -> Vec<f64>;

    /// 不經詞圖，直接從按鍵生成整句，最好的在前。生成不了（字級模型沒有這個能力、或模型出錯）返回空 Vec。
    ///
    /// 詞圖只會把整段按鍵讀成拼音，中英混輸（`yongdockerbushuhenfangbian`）與生詞在它那裡根本沒有路徑；
    /// P2C 訓練時見過的就是「按鍵 → 漢字」，這條路不受詞圖的讀法限制。
    fn generate(&self, _keys: &str, _beam: usize, _max_chars: usize) -> Vec<String> {
        Vec::new()
    }
}
