//! 形碼（五筆）：編碼直接查碼錶，不走拼音的切分與整句。

use super::*;

/// 五筆 86 的一小段：一級簡碼 V 是 發，`ggll` 是 一。
/// `甲` 的編碼故意取成 `kai`、`乙丙` 取成 `kaik`——與拼音的 開 同形，用來驗混輸時的排序。
const CODES: &str = "一\tggll\t90000\n发\tv\t30000\n到\tgc\t20000\n来\tgo\t15000\n开\tga\t5000\n开发\tgant\t900\n甲\tkai\t1000\n乙丙\tkaik\t800\n";

/// 只用形碼：拼音側關掉，候選只從碼錶來。
fn wubi() -> Engine {
    let mut engine = engine();
    engine.set_code_table(Some(CodeTable::parse(CODES).unwrap()));
    engine.set_phonetic(false);
    engine
}

/// 混輸：形碼與拼音兩邊都出候選。
fn mixed() -> Engine {
    let mut engine = engine();
    engine.set_code_table(Some(CodeTable::parse(CODES).unwrap()));
    engine
}

fn code_texts(engine: &mut Engine, input: &str) -> Vec<String> {
    engine.set_input(input);
    texts_of(engine)
}

#[test]
fn looks_up_by_code_prefix_and_prefers_the_finished_code() {
    let mut engine = wubi();
    // 編碼打全的 開 排在同字首的 開發 前面
    assert_eq!(code_texts(&mut engine, "ga"), ["开", "开发"]);
    // 沒打全時只按詞頻
    assert_eq!(
        code_texts(&mut engine, "g"),
        ["一", "到", "来", "开", "开发"]
    );
    // preedit 原樣顯示敲的編碼，沒有拼音切分
    engine.set_input("ga");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "ga");
    assert_eq!(query.tail, "ga");
    assert!(query.segmentations.is_empty());
    assert!(!query.decoded_keys);
    assert!(
        query
            .candidates
            .items
            .iter()
            .all(|c| c.syllables.is_empty())
    );
}

#[test]
fn commits_the_whole_code_and_empties_the_buffer() {
    let mut engine = wubi();
    engine.set_input("ga");
    let kai = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(kai.kind, CandidateKind::Code);
    assert_eq!(engine.commit(&kai), "开");
    assert!(engine.composition().is_empty());
}

#[test]
fn takes_no_part_in_the_pinyin_machinery() {
    let mut engine = wubi();
    // `v` 是字根鍵，不當表示式模式的字首
    assert_eq!(code_texts(&mut engine, "v"), ["发"]);
    // 快捷候選按拼音鍵認，形碼下不出
    assert!(code_texts(&mut engine, "rq").is_empty());
    // 模糊音不參與：`ga` 不會命中並不存在的 `ka`
    engine.set_fuzzy(FuzzyRules::ALL);
    assert_eq!(code_texts(&mut engine, "ga"), ["开", "开发"]);
}

#[test]
fn a_word_chosen_under_a_code_rises_next_time() {
    let mut engine = wubi().with_learner(Box::new(WordLearner::default()));
    engine.set_input("g");
    let kaifa = engine
        .query()
        .unwrap()
        .candidates
        .items
        .last()
        .cloned()
        .unwrap();
    assert_eq!(kaifa.text, "开发");
    engine.commit(&kaifa);
    engine.set_input("g");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发");
}

#[test]
fn code_candidates_get_translations_like_pinyin_ones() {
    let mut engine = wubi().with_translator(Box::new(FixedTranslator));
    engine.set_input("gant");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap();
    let senses = kaifa.translation.as_ref().unwrap().senses();
    assert_eq!(senses[0].text, "develop");
}

#[test]
fn code_commits_still_feed_translations_vocabulary_and_usage() {
    // 形碼換的是「怎麼按鍵出候選」，不是「上屏之後算什麼」：釋義標註、生詞判定、詞彙記錄與輸入統計
    // 都按上屏的詞工作，與方案無關。這條用例把這句話釘住。
    let usage = Arc::new(Mutex::new(Vec::new()));
    let vocabulary = Arc::new(Mutex::new(VocabularyCounts::default()));
    let mut engine = wubi()
        .with_translator(Box::new(FixedTranslator))
        .with_usage_meter(Box::new(MemoryMeter(usage.clone())))
        .with_vocabulary_tracker(Box::new(MemoryVocabulary(vocabulary.clone())));

    engine.set_input("gant");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let kaifa = query.candidates.items[0].clone();
    assert_eq!(kaifa.text, "开发");
    // 釋義照常標註；這條譯詞一個輪次都沒見過，標成生詞
    let sense = &kaifa.translation.as_ref().unwrap().senses()[0];
    assert_eq!(sense.text, "develop");
    assert!(sense.fresh);

    engine.commit(&kaifa);
    // 輸入統計：一箇中文詞、兩個漢字（形碼一次上屏就是一個詞）
    let usage = usage.lock().unwrap();
    assert_eq!(usage.len(), 1);
    assert_eq!((usage[0].words, usage[0].hanzi), (1, 2));
    // 詞彙記錄：上屏帶譯詞的中文候選，那條譯詞記「上屏過」；沒按修飾鍵打出來過，不是「用過」
    let counts = vocabulary.lock().unwrap();
    let entry = counts
        .get(&(Language::English, "develop".to_owned()))
        .expect("译词应当进了词汇记录");
    assert_eq!((entry.1, entry.2), (1, 0));
}

#[test]
fn mixed_input_orders_finished_codes_then_pinyin_then_code_prefixes() {
    let mut engine = mixed();
    // `kai`：編碼打全的 甲 在最前，拼音的 開 一族居中，只命中字首的 乙丙（`kaik`）墊後
    let texts = code_texts(&mut engine, "kai");
    assert_eq!(texts.first().map(String::as_str), Some("甲"));
    let position = |text: &str| texts.iter().position(|t| t == text).unwrap();
    assert!(position("开") < position("乙丙"));
    // `ka`：沒有打全的編碼，拼音的首選不被字首命中的形碼詞頂掉
    let texts = code_texts(&mut engine, "ka");
    assert_ne!(texts.first().map(String::as_str), Some("甲"));
    assert!(texts.contains(&"甲".to_owned()));
}

#[test]
fn mixed_input_keeps_the_shifted_mode_letters() {
    // 混輸下小寫 v / u 是字根鍵，表示式與問字同雙拼一樣改用大寫進
    let mut engine = mixed();
    assert!(engine.takes_mode_letter('V') && engine.takes_mode_letter('U'));
    engine.set_input("V1+2");
    assert!(engine.expression_mode());
    engine.set_input("v");
    assert!(!engine.expression_mode());
    assert_eq!(
        code_texts(&mut engine, "v").first().map(String::as_str),
        Some("发")
    );
}

#[test]
fn turning_the_pinyin_side_off_leaves_only_the_code_table() {
    let mut engine = wubi();
    assert_eq!(code_texts(&mut engine, "kai"), ["甲", "乙丙"]);
}

#[test]
fn a_longer_input_drops_the_code_side_by_itself() {
    // 五筆碼最長 4 位：從第 5 個字母起形碼查不到東西，混輸下自然只剩拼音
    let mut engine = mixed();
    let texts = code_texts(&mut engine, "kaifa");
    assert!(texts.contains(&"开发".to_owned()));
    assert!(!texts.contains(&"甲".to_owned()));
}

#[test]
fn mixed_input_falls_back_to_the_code_side_when_pinyin_cannot_read_it() {
    // `ggll` 切不成音節，但碼錶有 一：混輸不該因為拼音讀不出來就整個失敗
    let mut engine = mixed();
    assert_eq!(code_texts(&mut engine, "ggll"), ["一"]);
}

#[test]
fn keys_outside_the_code_alphabet_fall_back_to_raw() {
    let mut engine = wubi();
    engine.set_input("no-way");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "no-way");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::English);
}

#[test]
fn letters_never_start_a_mode_under_wubi() {
    // 形碼下每個字母都是字根鍵，連大寫也讓位（雙拼那套 Shift+V / Shift+U 在這裡不適用）：
    // 字母一律進緩衝區，表示式與問字模式只剩 `?` 這一個人口。
    let mut engine = wubi();
    assert!(!engine.takes_mode_letter('V') && !engine.takes_mode_letter('U'));
    engine.set_input("V1+2");
    assert!(!engine.expression_mode());
    engine.set_input("Usangemu");
    assert!(!engine.question_mode());
    engine.set_mode_keys(ModeKeys {
        question_mark: true,
        ..ModeKeys::default()
    });
    engine.set_input("?nihao");
    assert!(engine.question_mode());
}

#[test]
fn switching_back_to_pinyin_drops_the_code_table() {
    let mut engine = wubi();
    engine.set_code_table(None);
    assert!(!engine.is_code_mode());
    engine.set_input("kaifa");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发");
}
