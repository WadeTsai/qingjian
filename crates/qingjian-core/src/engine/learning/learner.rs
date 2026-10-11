use qingjian_dictionary::{Dictionary, WordList};

use super::Forgotten;
use crate::candidate::Candidate;
use crate::sentence::{Context, UserNgram};

/// 使用者詞頻學習。
pub trait Learner: Send {
    /// 使用者上屏了某個候選。
    fn record(&mut self, candidate: &Candidate);

    /// 該詞被使用者選擇過的次數，沒記錄返回 0。
    fn weight(&self, text: &str) -> u32;

    /// 使用者在輸入串 `input`（候選覆蓋的那段拼音，不含分隔符）下選了 `text`。
    /// 詞級排序裡同一輸入串下選過的詞排最前（`mgs` 選過 美國式，下次 `mgs` 它就是首選），與不分輸入的 [`Self::weight`] 分開記：
    /// `ba` 下選的是 吧，`bazhege` 下選的是 把，混在一起數就分不清。
    fn record_choice(&mut self, _input: &str, _text: &str) {}

    /// `text` 在輸入串 `input` 下被選過的次數，沒記錄返回 0。
    fn choice_weight(&self, _input: &str, _text: &str) -> u32 {
        0
    }

    /// 使用者對輸入串 `input` 按了回車原樣上屏，而當時拼寫糾錯正生效：這個串就是要原樣打的，以後不糾。
    fn record_raw(&mut self, _input: &str) {}

    /// `input` 被原樣上屏過幾次（見 [`Self::record_raw`]）。
    fn raw_count(&self, _input: &str) -> u32 {
        0
    }

    /// 撤銷一次 [`Self::record`]：使用者上屏後馬上整個刪掉重選了別的詞，剛才那次不算數。
    fn unrecord(&mut self, _text: &str) {}

    /// 撤銷一次 [`Self::record_choice`]。
    fn unrecord_choice(&mut self, _input: &str, _text: &str) {}

    /// 撤銷 `times` 份 [`Self::record_transition`]。
    fn unrecord_transition(&mut self, _context: Context<'_>, _word: &str, _times: u32) {}

    /// 記一個詞庫裡沒有的詞（來自雲聯想或以後的自動造詞），帶全拼音節；下次直接從本地出。
    fn learn_word(&mut self, _text: &str, _syllables: &[String]) {}

    /// 使用者片語成的小詞庫，與主詞庫一起查；沒有就返回 `None`。
    fn user_words(&self) -> Option<&Dictionary> {
        None
    }

    /// 使用者原樣上屏了一個像英文詞的字母串（中文模式按回車、英文模式空格 / 回車直通），或選了一個英文候選：
    /// 記進個人英文詞表，下次它就是英文候選，而且排在隨包詞表的同形詞前面。隨包詞表裡沒有的詞（`gist`）只能靠這裡學。
    fn learn_english(&mut self, _word: &str) {}

    /// 個人英文詞表，與隨包詞表一起給英文候選（它在前）；沒有就返回 `None`。
    fn user_english(&self) -> Option<&WordList> {
        None
    }

    /// 記一條詞序列轉移：`word` 在上文 `context`（前一個詞與再前一個詞，句首都是 `None`）之後上屏。
    /// 整句上屏按路徑上的詞逐條記，連續選詞上屏也記；喂個人 n-gram（二元與三元一起記）。
    /// `times` 是這次記幾份：使用者自己點選的詞記雙份（[`EXPLICIT_TRANSITION_WEIGHT`]），整句路徑裡順帶的記一份，
    /// 否則一次誤按空格上屏的整句要使用者改選兩次才能翻回來。
    ///
    /// [`EXPLICIT_TRANSITION_WEIGHT`]: super::EXPLICIT_TRANSITION_WEIGHT
    fn record_transition(&mut self, _context: Context<'_>, _word: &str, _times: u32) {}

    /// 個人 n-gram，整句轉換與詞級排序用它與靜態模型插值；沒有就返回 `None`。
    fn user_ngram(&self) -> Option<&UserNgram> {
        None
    }

    /// 使用者接受了一處音節級的敲錯糾正：把 `typed`（敲的那段字母）當成了 `intended`（候選的音節）上屏。
    /// 記進個人敲錯表，以後詞圖裡這條邊更便宜（見 `correction::TypoCosts::typo_cost`）。
    fn record_typo(&mut self, _typed: &str, _intended: &str) {}

    /// 撤銷一次 [`Self::record_typo`]。
    fn unrecord_typo(&mut self, _typed: &str, _intended: &str) {}

    /// `typed` 被當成 `intended` 接受過幾次。
    fn typo_count(&self, _typed: &str, _intended: &str) -> u32 {
        0
    }

    /// 使用者要求刪掉這個詞：它是使用者詞就刪掉，同時清掉選擇次數、各輸入串下的選擇、個人 n-gram 裡與它有關的轉移。
    /// 返回實際清掉了什麼。
    fn forget(&mut self, _text: &str) -> Forgotten {
        Forgotten::default()
    }

    /// 使用者要求刪掉一個個人英文詞。返回是否真有這個詞。
    fn forget_english(&mut self, _word: &str) -> bool {
        false
    }

    /// 把學習資料落盤。殼在退出或停用輸入法時呼叫；記憶體實現留空即可。
    ///
    /// 失敗只記日誌不返回錯誤：輸入優先於學習，持久化失敗不該影響殼的流程。
    fn flush(&mut self) {}
}

/// 不學習。
#[derive(Debug, Clone, Copy, Default)]
pub struct NoLearner;

impl Learner for NoLearner {
    fn record(&mut self, _candidate: &Candidate) {}

    fn weight(&self, _text: &str) -> u32 {
        0
    }
}
