//! Engine：Core 對外的唯一門面。
//!
//! 平臺層只跟這裡打交道：喂按鍵、拿候選、上屏。翻譯與學習通過 trait 注入，
//! 預設實現都是空操作，所以單元測試和 CLI 不需要真實詞典也能跑。

mod alignment;
mod annotation;
mod aux_code;
mod commit;
mod composing;
mod correcting;
mod decoded;
mod extras;
mod gloss;
mod input_log;
mod learning;
mod marked;
mod mode_keys;
mod prediction;
mod privacy;
mod query;
mod raw;
mod rescoring;
mod session;
mod setup;
mod statistics;
mod timings;
mod translator;
mod vocabulary;

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use qingjian_dictionary::{AuxCodeLookup, CodeTable, Dictionary, Match, WordList};

use crate::zhuyin::ZhuyinLayout;

pub use alignment::Alignment;
pub use annotation::AnnotationReport;
pub use aux_code::is_valid_aux_code_key;
pub use commit::{LastCommit, Transition};
pub use gloss::{FilledGloss, GlossFiller, NoGlossFiller};
pub use input_log::{
    CommitEntry, INPUT_LOG_VERSION, InputLogEntry, InputLogger, InputSource, LOGGED_CANDIDATES,
    NoInputLogger,
};
pub use learning::{Forgotten, Learner, NoLearner};
pub use marked::{AuxSegment, MarkedKind, MarkedSegment};
pub use mode_keys::{ModeKeys, QUESTION_PREFIX};
pub use prediction::{
    CloudWord, NoPredictor, Prediction, PredictionKind, PredictionPolicy, PredictionRequest,
    Predictor, SurroundingText,
};

pub use query::Query;
pub use raw::RawPreedit;
pub use session::EngineSession;
pub use statistics::{BOOKS, Book, NoUsageMeter, Usage, UsageMeter, UsageSummary, book_scale};
pub use timings::Timings;
pub use translator::{NoTranslator, Translator};
pub use vocabulary::{
    FRESH_UNTIL, LevelCount, NoVocabularyTracker, VocabularySummary, VocabularyTracker,
};

use crate::candidate::{Candidate, CandidateKind, CandidateList, Language};
use crate::composition::Composition;
use crate::correction::{self, Correction, TypoCosts, typo};
use crate::emoji::EmojiTable;
use crate::english;
use crate::fuzzy::{Expanded, FuzzyRules};
use crate::history::InputHistory;
use crate::parser::{self, ParseError, Segmentation};
use crate::punctuation::Punctuation;
use crate::ranking::{self, Scored};
use crate::sentence::{
    self, Conversion, Interpolation, LanguageModel, NoLanguageModel, Personal, SentenceScorer,
};
use crate::shortcut;
use crate::shuangpin::Scheme;

use commit::CommitChain;

pub struct Engine {
    /// 靜態詞庫。
    dictionary: Dictionary,

    /// 譯文提供方，預設為 [`NoTranslator`]。
    translator: Box<dyn Translator>,

    /// 字首模式鍵（表示式 / 問字）。
    modes: ModeKeys,

    /// 附加詞庫（領域詞庫、使用者匯入的），與主詞庫一起查詞、一起進整句詞圖；不參與語言模型（它們沒有 bigram，
    /// 走詞頻兜底）。殼按使用者目錄 `dicts/` 與配置 `[dictionaries]` 裝配。
    extra_dictionaries: Vec<Dictionary>,

    /// 英文候選的釋義（英→中），預設為 [`NoTranslator`]。英文候選的輔助語言是主語言中文，
    /// 與中文候選查學習語言的表分開，仍是「一個候選只顯示一種輔助語言」。
    english_translator: Box<dyn Translator>,

    /// 使用者詞頻，預設為 [`NoLearner`]；私密輸入期間只讀不寫（[`learning::MutedLearner`]）。
    learner: learning::MutedLearner,

    /// 當前拼音緩衝區。
    composition: Composition,

    /// 英文詞表，中英混輸用；沒有就不出英文候選。
    english: Option<WordList>,

    /// 英文模式（殼裡 Caps Lock 亮著）：緩衝區裡的字母不當拼音，候選來自英文詞表的補全與糾正。
    english_mode: bool,

    /// 全形標點與引號配對狀態。
    punctuation: Punctuation,

    /// 中文標點轉換開關。
    full_width_punctuation: bool,

    /// 使用者定義的固定位置文本。
    custom_phrases: Vec<crate::CustomPhrase>,

    /// 候選後面配不配 emoji（配置 `[general] emoji`，預設開）；關了 emoji 表照樣載入，只是不出候選。
    emoji_candidates: bool,

    /// 中文模式下整段是英文詞（或像英文詞的開頭）時給不給英文候選與補全（配置 `[general] english_in_chinese`，預設開）。
    english_in_chinese: bool,

    /// 中英混輸時中文候選總在英文詞前面（預設關：拼音不像話的輸入英文詞排第一，常在中文模式裡打英文詞的人靠它）。
    chinese_first: bool,

    /// 中文模式下 Shift+字母進組句緩衝區（配置 `[general] shift_letter = "compose"`，預設關）。
    /// 關著由殼直接把大寫字母交給應用，Core 這一路就不該收——否則 `Cpan` 這種會被當成拼音。
    shift_letter_compose: bool,

    /// 聯想提供方，預設為 [`NoPredictor`]。
    predictor: Box<dyn Predictor>,

    /// 整句轉換的語言模型，預設為 [`NoLanguageModel`]（退化成一元詞頻）。
    language_model: Box<dyn LanguageModel>,

    /// 整句路徑的同步神經重打分器（字級 Transformer，查詢裡當場打分；CLI 評測用）。
    sentence_scorer: Option<Box<dyn SentenceScorer>>,

    /// 非同步重打分：後臺執行緒裡的打分器，殼在停頓後送任務、輪詢結果（見 [`rescoring`]）。
    rescorer: Option<rescoring::RescoreWorker>,

    /// 「前文 + 整句文本 → 神經分」快取，同步與非同步打分共用。
    neural_cache: std::cell::RefCell<rescoring::NeuralCache>,

    /// 殼給的應用裡游標前的文本；`None` 時前文用本會話歷史。
    rescoring_before: Option<String>,

    /// 重打分時神經得分的權重 λ：最終分 = 路徑分 + λ·(神經分 − 靜態分)。
    neural_weight: f64,

    /// 只有路徑分與最優路徑差距在這麼多 nat 以內的路徑才參與重排：差距大的多半是個人 n-gram 拉開的，通用模型不該翻盤。
    neural_margin: f64,

    /// 重打分給模型看的前文長度（本會話最近上屏的字元數），0 為不給前文。
    neural_context: usize,

    /// 個人 n-gram 與靜態模型插值的引數；只有回放調參會改（`set_interpolation`），殼用預設值。
    interpolation: Interpolation,

    /// 敲錯糾正的代價；同上，只有回放調參會改（`set_typo_costs`）。
    typo_costs: TypoCosts,

    /// 最近幾次上屏各記了哪些學習、之後退格了幾個字；使用者把它們刪掉重選時把學習退回去（見 [`Self::note_backspace`]）。
    /// 最新的在末尾，最多留 [`RECENT_COMMITS`] 條。
    recent_commits: Vec<LastCommit>,

    /// 本次 commit 裡記下的詞轉移，commit 結束時搬進 `last_commit`。
    recording: Vec<Transition>,

    /// 輸入日誌的落盤方；預設不記，私密輸入期間一律不記（[`input_log::MutedLogger`]）。
    logger: input_log::MutedLogger,

    /// 私密輸入中（見 [`Self::set_private`]）：不學、不記、不發雲端。
    private: bool,

    /// 輸入日誌條目的序號。
    log_sequence: u64,

    /// 最近一次查詢的候選順序是否經過神經重排（`rescore_paths` 置位，`query` 開頭清零），寫進輸入日誌。
    last_rescored: std::cell::Cell<bool>,

    /// 這段組句裡第一次退格前的緩衝區：上屏時與最終鍵串不同就記一條 `retype`。
    retype_snapshot: Option<String>,

    /// 組句外直通給應用的字元，攢到下一次上屏或上文斷開時寫成一條 `passthrough`。
    passthrough_pending: String,

    /// 這段組句翻了幾頁候選。
    page_turns: u32,

    /// 這段組句第一鍵的時刻（算首鍵到上屏的毫秒）。
    composition_started: Option<Instant>,

    /// 正在輸入的應用標識，殼在焦點變化時給；寫進輸入日誌。
    application: Option<String>,

    /// 上次記 `break` 之後有沒有上屏過：沒有就不再記，免得失焦一次記一條。
    committed_since_break: bool,

    /// 最近一次聯想請求時的作用域：結果可能在上屏之後才到，日誌裡要記請求時的拼音。
    last_prediction_scope: String,

    /// 輸入統計的累計方（打了多少字）；預設不記。
    meter: Box<dyn UsageMeter>,

    /// 學習語言的詞彙記錄（見過 / 上屏過哪些譯詞）；預設不記也不標生詞。
    vocabulary: Box<dyn VocabularyTracker>,

    /// 釋義兜底：釋義表裡沒有的詞上屏後問雲端；預設不問。
    gloss_filler: Box<dyn GlossFiller>,

    /// 候選視窗當前頁上的譯詞（殼每次畫完告知），上屏時記成「看到過」。
    displayed: Vec<(Language, String)>,

    /// 上一次查詢的摘要，上屏時寫進輸入日誌。
    last_query: std::cell::RefCell<Option<query::QuerySnapshot>>,

    /// 上一次算過的拼寫糾正：(作用域, 結果)。query 算一次，commit / take_raw 複用，別再跑一遍變體列舉。
    correction_cache: std::cell::RefCell<Option<(String, Option<Correction>)>>,

    /// 整句轉換的格子候選快取：跨按鍵複用，學習資料一變就清（見 [`Self::forget_span_cache`]）。
    span_cache: std::cell::RefCell<sentence::SpanCache>,

    /// 本次會話經我們上屏的文本，應用不給上下文時用它聯想。
    history: InputHistory,

    /// 最近一次聯想請求的序號，0 表示還沒發過。
    prediction_sequence: u64,

    /// 最近一次聯想請求的種類：只有組句聯想的結果要按拼音校驗。
    last_prediction_kind: PredictionKind,

    /// 最近一次問字請求裡本地把問題拼音轉成的漢字，用來剔掉模型複述問題的「答案」。
    last_question_guess: String,

    /// 連續上屏的鏈，個人 n-gram 與自動造詞靠它。
    chain: CommitChain,

    /// 模糊音開關，預設全關。
    fuzzy: FuzzyRules,

    /// 雙拼方案，`None` 為全拼。開著時緩衝區裡是雙拼鍵，查詞前先解成全拼（見 [`crate::shuangpin`]）。
    shuangpin: Option<Scheme>,

    /// 雙拼方案下 preedit 顯示原始按鍵（如 `ljse`）還是展開成全拼（`lan'se`）。
    shuangpin_raw_preedit: bool,

    /// 注音模式：`Some` 時緩衝區裡是注音鍵位，按這套排列解成符號再解成拼音（見 [`crate::zhuyin`]）；`None` 關。
    zhuyin: Option<ZhuyinLayout>,

    /// 注音鍵盤排列（`[general] zhuyin_layout`）。單獨記一份：關掉再開注音時排列不丟。
    zhuyin_layout: ZhuyinLayout,

    /// 形碼碼錶（五筆）。`Some` 時編碼參與查詢，按字首查表（見 [`Engine::query_code`]）。
    code: Option<CodeTable>,

    /// 拼音側（全拼 / 雙拼 / 注音）參不參與查詢，預設參與。
    ///
    /// 與 `code` 組合出三種情形：只有拼音（形碼關）、只有形碼（拼音關，`[general] scheme = "none"`）、
    /// **兩邊都開 = 混輸**（編碼打全的形碼候選在前，見 [`Engine::query_mixed`]）。兩個都關著時按拼音走。
    phonetic: bool,

    /// emoji 表，沒有就不出 emoji 候選。
    emoji: Option<EmojiTable>,

    /// 輔碼態：`None` 是拼音態，`Some` 是輔碼態（空串 = 剛敲下觸發鍵、碼段還沒開始）。
    /// 碼段不進 `composition`：它與拼音分段記賬、分段畫（見 [`AuxSegment`]）。
    aux_code: Option<String>,

    /// 輔碼總開關（配置項 `[aux_code] enabled`，預設關）：關著時觸發鍵不進輔碼態、純拼音態也不掛碼。
    aux_enabled: bool,

    /// 候選上是否顯示碼（配置項 `[general] aux_code_show`，預設關）：純拼音態逐候查首條碼的短路開關，
    /// 由殼裝配時告知（輔碼態不受它管，看碼有引導意義）。
    aux_show: bool,

    /// 進輔碼態的觸發鍵，配置項 `[general] aux_code_key`，預設 [`DEFAULT_AUX_CODE_KEY`]。
    aux_code_key: char,

    /// 碼段刪空後是否留在輔碼態（配置項 `[general] aux_code_keep_empty`，預設開）：開 = 刪空停在
    /// 輔碼態（`;` 仍在、候選全回），空碼段再按一次退格才退出；關 = 刪空即回拼音態。
    aux_keep_empty: bool,

    /// 輔碼碼錶，殼按使用者目錄 `codes/` 與配置裝配；空表示沒裝碼錶（輔碼態篩不出任何詞）。
    aux_codes: Vec<Arc<dyn AuxCodeLookup>>,

    /// 繁體輸出模式。
    traditional: bool,

    /// 繁體轉換器。
    opencc: Option<ferrous_opencc::OpenCC>,

    /// 繁體輸出時「繁體 → 原簡體」的對映，組句結束清空；學習、譯詞、撤銷都按簡體原文走。
    traditional_map: std::cell::RefCell<HashMap<String, String>>,
}

/// 形碼編碼最長几位（五筆四碼）：混輸下超過它的輸入只可能是拼音。
const MAX_CODE_LENGTH: usize = 4;

/// 英文補全最多幾條（`compa` → company / compare / …）。
const ENGLISH_COMPLETIONS: usize = 3;

/// 原樣上屏的字母串至少幾個字母才當英文詞學：單字母（`a`、`I`）不值得記。
const MIN_ENGLISH_WORD_LETTERS: usize = 2;

/// 原樣上屏的字母串最多幾個字母才當英文詞學：再長的多半不是一個詞，是一整句打不出來的輸入
/// （`yongdockerbushuhenfangbian`），學進去會反過來堵住混輸那條路。
/// 15 是 2026-09-25 按資料定的：使用者個人表 1102 條裡超過 15 個字母的 9 條全是失敗的拼音、無一真詞；
/// 隨包英文詞表 94568 條裡超過 15 個字母的只佔 0.69%，而那些詞基本都已經在隨包表裡、不靠學。
/// 英文模式下不設這個上限：那時敲的本來就是英文。
const MAX_ENGLISH_WORD_LETTERS: usize = 15;

/// 英文模式一次最多給幾條候選：兩頁足夠，再往後沒人翻。
const ENGLISH_MODE_CANDIDATES: usize = 18;

/// 英文補全至少要幾個字母：太短的字首誰都像。
const MIN_COMPLETION_LETTERS: usize = 3;

/// 整段末尾當英文詞的尾段至少幾個字母，前面的拼音頭至少幾個字母（見 `query::EnglishTail`）。
const MIN_ENGLISH_TAIL_LETTERS: usize = 2;
const MIN_ENGLISH_TAIL_HEAD_LETTERS: usize = 2;

/// 尾段自己也是合法拼音時（`fan`、`database`）至少幾個字母才考慮英文讀法：三個字母的拼音音節太多。
const MIN_PINYIN_LIKE_TAIL_LETTERS: usize = 4;

/// 句中切到英文的代價（log 機率）：尾段像拼音時英文讀法要比拼音讀法高出這麼多才勝出。拍的，攢夠日誌後用 `--replay` 調。
const ENGLISH_SWITCH_PENALTY: f64 = 3.0;

/// 英文詞頻的下限（Zipf）：沒有詞頻的詞按百萬分之一算。
const ENGLISH_ZIPF_FLOOR: f64 = 3.0;

/// emoji 只配給前幾個候選，每個詞最多幾個、一次最多幾個。
const EMOJI_SCAN: usize = 5;
const EMOJI_PER_WORD: usize = 2;
const EMOJI_TOTAL: usize = 3;

/// 自動造詞：使用者連著選出的兩個詞，合起來不在詞庫裡、且這條接續已記過這麼多次，就記成使用者詞。
/// 同一段拼音裡連著選出來的（`qingjian` 選 青 再選 簡）是「使用者把它當一個詞打」的強訊號，兩次就夠；
/// 第一次可能是誤選或偶然。
const AUTO_WORD_THRESHOLD_SAME_BUFFER: u32 = 2;

/// 分兩段打的（`qing` 選 青、再打 `jian` 選 簡）訊號弱一些，要三次，免得 了我 這類虛詞接續也成詞。
const AUTO_WORD_THRESHOLD: u32 = 3;

/// 退格撤銷最多回看幾次上屏：刪掉「沃德 書」兩個詞再重打時，要能找到兩個詞之前的那一次。
const RECENT_COMMITS: usize = 4;

/// 自動造出的詞最多幾個字：再長就不是詞而是短語了。
const AUTO_WORD_MAX_CHARS: usize = 4;

/// 使用者自己點選的詞，轉移記幾份；整句路徑裡順帶的記一份。
/// 整句是模型自己算出來的，按空格接受它會把這條路徑喂回模型，形成自我強化；使用者明確改選的詞要能壓過這種回聲。
pub const EXPLICIT_TRANSITION_WEIGHT: u32 = 2;

/// 拼音短於這個字母數不聯想：一兩個字母的意圖太模糊，白花一次請求。
const MIN_PREDICTION_LETTERS: usize = 2;

/// 輔碼觸發鍵的預設值（`[general] aux_code_key`）。
pub const DEFAULT_AUX_CODE_KEY: char = ';';

/// 隨聯想請求附帶的本地候選條數。
const PREDICTION_CANDIDATE_HINTS: usize = 5;

/// 一次查詢最多給殼多少條候選。同音字最多的音節也不到這個數，再往後都是長詞，沒人會翻到。
const MAX_CANDIDATES: usize = 500;

/// 神經重打分看 Viterbi 的前幾條路徑：束寬是 8，再多也沒有。
const RESCORE_PATHS: usize = 6;

/// 長輸入最多給幾條整句候選（重排後的前幾條路徑）：一條不夠用，長句錯一個字就得拆開重打。
const SENTENCE_CANDIDATES: usize = 3;

/// 短於這麼多音節的不給備選整句：那時候選表裡的詞級候選比另一種讀法有用。
const ALTERNATE_MIN_SYLLABLES: usize = 4;

/// 短於這麼多字母的不讓模型直接生成整句：那麼短的輸入詞級候選夠用，生成的幾十毫秒不值。
const MIN_GENERATED_LETTERS: usize = 6;

/// 神經重打分的預設權重 λ（見 `Engine::neural_weight`）：整句評測集上 0.5 到 1.0 一樣好、0.75 最高（見 docs/notes/neural-rescoring.md），
/// 取 0.5 給個人 n-gram 留餘量；回放裡看到的「λ 大整句掉」是那把尺子的偏差。
pub const NEURAL_WEIGHT: f64 = 0.5;

/// 神經重打分的預設門檻（nat）：路徑分落後最優路徑超過這麼多的不參與重排。預設不設（4 nat 試過沒幫助），留作調參的旋鈕。
pub const NEURAL_MARGIN: f64 = f64::INFINITY;

/// 重打分給模型看的前文：本次會話最近上屏的這麼多個字元。
pub const RESCORE_CONTEXT_CHARS: usize = 64;

impl Engine {
    pub fn new(dictionary: Dictionary) -> Self {
        Self {
            dictionary,
            extra_dictionaries: Vec::new(),
            translator: Box::new(NoTranslator),
            english_translator: Box::new(NoTranslator),
            modes: ModeKeys::default(),
            learner: learning::MutedLearner::new(Box::new(NoLearner)),
            composition: Composition::default(),
            english: None,
            english_mode: false,
            punctuation: Punctuation::default(),
            full_width_punctuation: true,
            custom_phrases: Vec::new(),
            emoji_candidates: true,
            english_in_chinese: true,
            chinese_first: false,
            shift_letter_compose: false,
            predictor: Box::new(NoPredictor),
            language_model: Box::new(NoLanguageModel),
            sentence_scorer: None,
            rescorer: None,
            neural_cache: std::cell::RefCell::new(rescoring::NeuralCache::default()),
            rescoring_before: None,
            neural_weight: NEURAL_WEIGHT,
            neural_margin: NEURAL_MARGIN,
            interpolation: Interpolation::DEFAULT,
            typo_costs: TypoCosts::DEFAULT,
            neural_context: RESCORE_CONTEXT_CHARS,
            correction_cache: std::cell::RefCell::new(None),
            span_cache: std::cell::RefCell::new(sentence::SpanCache::default()),
            recent_commits: Vec::new(),
            logger: input_log::MutedLogger::new(Box::new(NoInputLogger)),
            private: false,
            log_sequence: 0,
            last_rescored: std::cell::Cell::new(false),
            retype_snapshot: None,
            passthrough_pending: String::new(),
            page_turns: 0,
            composition_started: None,
            application: None,
            committed_since_break: false,
            last_prediction_scope: String::new(),
            meter: Box::new(NoUsageMeter),
            vocabulary: Box::new(NoVocabularyTracker),
            gloss_filler: Box::new(NoGlossFiller),
            displayed: Vec::new(),
            last_query: std::cell::RefCell::new(None),
            recording: Vec::new(),
            history: InputHistory::default(),
            prediction_sequence: 0,
            last_prediction_kind: PredictionKind::Compose,
            last_question_guess: String::new(),
            chain: CommitChain::default(),
            fuzzy: FuzzyRules::default(),
            shuangpin: None,
            shuangpin_raw_preedit: false,
            zhuyin: None,
            zhuyin_layout: ZhuyinLayout::default(),
            code: None,
            phonetic: true,
            emoji: None,
            aux_code: None,
            aux_enabled: false,
            aux_show: false,
            aux_code_key: DEFAULT_AUX_CODE_KEY,
            aux_keep_empty: true,
            aux_codes: Vec::new(),
            traditional: false,
            opencc: None,
            traditional_map: std::cell::RefCell::new(HashMap::new()),
        }
    }
}

/// 緩衝區是否是英文直輸段：含拼音鍵與 `'` 以外的字元（`no-way`、`a.b`），且不是表示式 / 問字模式。
/// 微軟 / 搜狗雙拼下 `;` 也是拼音鍵；注音下按排列的鍵位表算，空格是一聲。
fn is_raw(
    text: &str,
    modes: ModeKeys,
    shuangpin: Option<Scheme>,
    zhuyin: Option<ZhuyinLayout>,
) -> bool {
    let is_key = |c: char| match (zhuyin, shuangpin) {
        (Some(layout), _) => layout.map_key(c).is_some(),
        (None, Some(scheme)) => scheme.is_key(c),
        (None, None) => c.is_ascii_lowercase(),
    };
    !text.is_empty()
        && !modes.is_expression(text, zhuyin)
        && !modes.is_question(text, zhuyin)
        && text.chars().any(|c| !(is_key(c) || c == '\''))
}

/// 命中是否靠模糊音：某個音節不被敲的那個模式接受。
/// 原樣上屏的字母串像不像一個英文詞：純 ASCII 字母、至少兩個。中文模式下還要求它**不能**切成完整的拼音
/// （`hao` 回車多半是要拼音字母本身，`gist` / `python` / `hello` 切不乾淨才是英文），
/// 且不能長過 [`MAX_ENGLISH_WORD_LETTERS`]；英文模式下敲的全是英文，兩條都不用判。
fn looks_like_english_word(raw: &str, english_mode: bool) -> bool {
    if raw.len() < MIN_ENGLISH_WORD_LETTERS || !raw.bytes().all(|b| b.is_ascii_alphabetic()) {
        return false;
    }
    english_mode
        || (raw.len() <= MAX_ENGLISH_WORD_LETTERS
            && !parser::is_fully_segmentable(&raw.to_ascii_lowercase()))
}

/// 模式的記憶化鍵：完整音節原樣，字首音節後加 `*`。
fn pattern_key(pattern: &[qingjian_dictionary::SyllablePattern<'_>]) -> String {
    let mut key = String::with_capacity(pattern.len() * 7);
    for p in pattern {
        key.push_str(p.text);
        if !p.complete {
            key.push('*');
        }
        key.push(' ');
    }
    key
}

/// 末尾 `count` 個字元。
fn take_last_chars(text: &str, count: usize) -> String {
    let total = text.chars().count();
    text.chars().skip(total.saturating_sub(count)).collect()
}

/// 開頭 `count` 個字元。
fn take_first_chars(text: &str, count: usize) -> String {
    text.chars().take(count).collect()
}

/// 游標後剩餘拼音的顯示形式：能切就按音節用 `'` 連上，切不動就原樣。
fn marked_rest(rest: &str) -> String {
    if rest.is_empty() {
        return String::new();
    }
    match segment_longest_prefix(rest) {
        Ok((segmentations, tail)) => query::join_marked(&segmentations, tail),
        Err(_) => rest.to_owned(),
    }
}

/// 整段切不動時退而求其次：找能切分的最長字首，剩餘字母作為尾部返回。
/// `kaifv` → (`kai f…` 的切分, `v`)。連第一個字母都切不動才報錯。
fn segment_longest_prefix(text: &str) -> Result<(Vec<Segmentation>, &str), ParseError> {
    match parser::segment(text) {
        Ok(segmentations) => Ok((segmentations, "")),
        Err(ParseError::NoSegmentation) => (1..text.len())
            .rev()
            .find_map(|end| {
                parser::segment(&text[..end])
                    .ok()
                    .map(|s| (s, &text[end..]))
            })
            .ok_or(ParseError::NoSegmentation),
        Err(error) => Err(error),
    }
}

/// 候選的音節序列在 `input` 開頭覆蓋了多少個位元組。音節之間允許有 `'`。
///
/// 每個音節吃掉輸入裡與它相同的最長字首：全拼 `kaifa` 的 開發 吃 5 個，簡拼 `kf` 的 開發 吃 2 個，
/// 未打完的 `kaif` 也吃完。吃不到任何字母說明候選與輸入的切分方式不一致，就此停止。
/// 按輸入串記選擇用的鍵：作用域開頭 `len` 個位元組裡的字母（去掉分隔符 `'`），
/// 查詢時按候選覆蓋的字母數擷取同一個串，兩邊才對得上。
fn choice_key(scope: &str, len: usize) -> String {
    scope[..len.min(scope.len())]
        .chars()
        .filter(|c| *c != '\'')
        .collect()
}

/// 切分裡非末尾的簡拼音節數，見 `ranking::Scored::abbreviated`。
fn abbreviated_count(patterns: &[qingjian_dictionary::SyllablePattern<'_>]) -> usize {
    patterns
        .iter()
        .rev()
        .skip(1)
        .filter(|p| !p.complete)
        .count()
}

#[cfg(test)]
mod tests;
