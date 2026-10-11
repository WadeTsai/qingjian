//! 詞圖讀不通整段輸入時的兜底候選：讓模型直接按按鍵生成整句。

use super::*;

impl Engine {
    /// 模型直接按整段按鍵生成的整句候選，最好的在前；不該生成或生成不出來時是空的。
    ///
    /// 只在**詞圖讀不通整段**時才走，三種讀不通：
    ///
    /// - 最優切分裡有讀不完整的音節（`yongdockerbushuhenfangbian` → `yong d… o c… ke r… bu shu…`）；
    /// - 切分壓根沒覆蓋到末尾（`woyongvscodexiedaima` 切到 `wo yong` 就停了，`v` 起不了音節）；
    /// - 拼寫糾錯生效了（`corrected`）——`womaileyigeiphone` 裡的 `phone` 被當成敲錯的 `paone`，
    ///   糾完切分就「乾淨」了，前兩條都看不出來。糾錯本來判的就是「這段不像拼音」，與這裡是同一個問題。
    ///
    /// 三種都說明整段裡有一截不是拼音：中英混輸的英文段，或者打得太離譜。這時詞圖只能把那一截
    /// 硬讀成拼音、丟掉後半句、或者糾成別的字，出來的必然是廢話（用的哦乘客仍不熟很方便 / 我用 /
    /// 我買了一給跑呢），生成的幾十毫秒花得值。
    ///
    /// 拼音本身乾淨的輸入不走這條路：詞圖在那兒可靠，而且每次查詢都生成會把按鍵回撥拖垮。
    /// 漏網的是「英文詞本身就是合法拼音、連糾錯都不用」那種（`zhegeapihenhaoyong` 的 `api` 讀成 `a pi`），
    /// 光看切分分不出來，要等整句評測給出「生成常態參與排序」的結論再說。
    ///
    /// 雙拼 / 注音不走：它們的按鍵不是模型訓練時見過的那套字母。
    pub(in crate::engine) fn generated_sentence_candidates(
        &self,
        best: &Segmentation,
        keys: &str,
        corrected: bool,
    ) -> Vec<Candidate> {
        if keys.len() < MIN_GENERATED_LETTERS
            || self.shuangpin.is_some()
            || self.zhuyin
            || !keys.bytes().all(|b| b.is_ascii_lowercase())
        {
            return Vec::new();
        }
        // 上面已經保證整段都是小寫字母，沒有 `'`，`letters()` 與位元組長度可比
        let covers_all = best.letters() == keys.len();
        if best.incomplete_count() == 0 && covers_all && !corrected {
            return Vec::new();
        }
        let mut out: Vec<Candidate> = Vec::new();
        for text in self.generated_sentences(keys) {
            // 一個漢字都沒有的（整段被當成英文原樣抄回來）不算候選：那條路英文詞表那邊本來就有
            if !text.chars().any(sentence::is_han) || out.iter().any(|c| c.text == text) {
                continue;
            }
            out.push(Candidate {
                text,
                kind: CandidateKind::Generated,
                syllables: Vec::new(),
                reading: None,
                translation: None,
                aux_code: None,
            });
        }
        out
    }
}
