//! 青簡輸入法核心。
//!
//! 平臺無關：詞庫、拼音解析、候選生成、排序、學習與翻譯的介面全部在這裡。
//! 平臺層（IMK / TSF / IBus-Fcitx）只負責把按鍵餵給 [`Engine`]、把候選畫出來。
//! 判斷標準：換掉 IMK 換成 TSF，不應該需要改這裡的任何一行。

pub mod candidate;
pub mod composition;
pub mod correction;
pub mod custom_phrase;
pub mod emoji;
pub mod engine;
pub mod english;
pub mod fuzzy;
pub mod history;
pub mod parser;
pub mod punctuation;
pub mod ranking;
pub mod sentence;
pub mod shortcut;
pub mod shuangpin;
pub mod storage;
pub mod zhuyin;

pub use custom_phrase::CustomPhrase;

pub use candidate::{
    Candidate, CandidateKind, CandidateLayout, CandidateList, Cell, GRID_ROWS, Grid, Language,
    MAX_CELL_EMS, PartOfSpeech, Sense, Translation,
};
pub use composition::Composition;
pub use correction::Correction;
pub use emoji::EmojiTable;
pub use engine::{
    AnnotationReport, AuxSegment, BOOKS, Book, CloudWord, CommitEntry, DEFAULT_AUX_CODE_KEY,
    Engine, EngineSession, FRESH_UNTIL, FilledGloss, Forgotten, GlossFiller, INPUT_LOG_VERSION,
    InputLogEntry, InputLogger, InputSource, Learner, LevelCount, MarkedKind, MarkedSegment,
    ModeKeys, NEURAL_MARGIN, NEURAL_WEIGHT, NoGlossFiller, NoInputLogger, NoLearner, NoPredictor,
    NoTranslator, NoUsageMeter, NoVocabularyTracker, Prediction, PredictionKind, PredictionPolicy,
    PredictionRequest, Predictor, QUESTION_PREFIX, Query, RESCORE_CONTEXT_CHARS, RawPreedit,
    SurroundingText, Timings, Translator, Usage, UsageMeter, UsageSummary, VocabularySummary,
    VocabularyTracker, book_scale, is_valid_aux_code_key,
};
pub use fuzzy::FuzzyRules;
pub use history::InputHistory;
pub use parser::{ParseError, Segmentation};
pub use punctuation::Punctuation;
pub use qingjian_dictionary as dictionary;
pub use shuangpin::Scheme as ShuangpinScheme;
pub use zhuyin::ZhuyinLayout;
