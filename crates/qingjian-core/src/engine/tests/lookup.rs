//! 查詞、切分、游標與上屏消耗。

use super::*;

#[test]
fn exact_word_first_then_longer_then_prefix_expansions_then_prefix_words() {
    assert_eq!(texts("kaifa"), ["开发", "开放", "开饭", "开发者", "开"]);
}

#[test]
fn partial_last_syllable_expands() {
    assert_eq!(texts("kaif"), ["开放", "开发", "开饭", "开发者", "开"]);
}

#[test]
fn initials_match_abbreviated_words_and_commit_consumes_letters() {
    let all = texts("kf");
    assert_eq!(all[0], "开放"); // 同為簡拼命中，按詞頻
    assert!(all.contains(&"开发".to_owned()) && all.contains(&"咖啡".to_owned()));

    let mut engine = engine();
    engine.set_input("kfzhe");
    let kaifa = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    engine.commit(&kaifa);
    assert_eq!(engine.composition().text(), "zhe");
}

#[test]
fn unparsable_tail_is_kept_aside() {
    let mut engine = engine();
    // kaiv 唯一的糾法是刪掉剛敲的 v，那不算糾錯：v 留作尾巴等下一鍵（kaifv 會被糾成 kaifa）
    engine.set_input("kaiv");
    let query = engine.query().unwrap();
    assert!(query.correction.is_none());
    assert_eq!(query.tail, "v");
    assert_eq!(query.marked_text(), "kai'v");
    assert_eq!(query.candidates.items[0].text, "开");

    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    engine.commit(&kaifa);
    assert_eq!(engine.composition().text(), "v");
    // 剩下的 v 進表示式模式：沒候選但也不報錯
    assert!(engine.query().unwrap().candidates.items.is_empty());
}

#[test]
fn cursor_edits_requery_from_the_start_and_map_into_marked_text() {
    let mut engine = engine();
    engine.set_input("kaifa");
    engine.move_cursor_left();
    engine.move_cursor_left();
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "kai'fa");
    assert_eq!(query.marked_cursor(), 3); // kai|'fa

    engine.push('n');
    let query = engine.query().unwrap();
    assert_eq!(query.text, "kainfa");
    assert_eq!(query.marked_cursor(), 5); // kai'n|'fa

    engine.set_input("xi'an");
    engine.move_cursor_left();
    engine.move_cursor_left();
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "xi'an");
    assert_eq!(query.marked_cursor(), 3); // xi'|an，緊跟使用者自己敲的 '
}

#[test]
fn punctuation_follows_committed_text() {
    let mut engine = engine();
    assert_eq!(engine.punctuate(','), Some("，"));
    engine.note_passthrough('3');
    assert_eq!(engine.punctuate('.'), None);
    engine.set_input("kaifa");
    let kaifa = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&kaifa);
    assert_eq!(engine.punctuate('.'), Some("。"));
}

#[test]
fn marked_text_joins_best_segmentation_with_apostrophes() {
    let mut engine = engine();
    engine.set_input("kaifa");
    assert_eq!(engine.query().unwrap().marked_text(), "kai'fa");
    engine.set_input("kf");
    assert_eq!(engine.query().unwrap().marked_text(), "k'f");
}

#[test]
fn prefix_words_appear_after_full_matches() {
    // 開放 / 開飯 來自 `kai f… a zhe` 這種切分的字首 `kai f…`，排在覆蓋更多字母的 開發 之後
    let all = texts("kaifazhe");
    assert_eq!(&all[..2], ["开发者", "开发"]);
    assert_eq!(all.last().map(String::as_str), Some("开"));
}

#[test]
fn ambiguous_segmentation_merges_results() {
    let all = texts("xian");
    assert_eq!(all[0], "先");
    assert!(all.contains(&"西安".to_owned()));
}

#[test]
fn complete_syllable_that_is_also_prefix_expands_after_exact() {
    // 下 是完整命中；先 / 想 來自 xia → xian / xiang 的字首擴充套件；西安 來自 xi + a… 的次級切分
    assert_eq!(texts("xia"), ["下", "先", "想", "西安"]);
}

#[test]
fn keyboard_u_umlaut_spelling_matches_canonical_dictionary_keys() {
    let dictionary =
        Dictionary::parse("策略\tce lve\t9000\n虐待\tnve dai\t8000\n学习\txue xi\t7000\n").unwrap();
    let mut engine = Engine::new(dictionary);

    engine.set_input("celue");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "ce'lue");
    assert_eq!(query.candidates.items[0].text, "策略");
    assert_eq!(query.candidates.items[0].syllables, ["ce", "lve"]);
    let strategy = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&strategy), "策略");
    assert!(engine.composition().is_empty());

    engine.set_input("nuedai");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "虐待");

    engine.set_input("xuexicelue");
    let sentence = &engine.query().unwrap().candidates.items[0];
    assert_eq!(sentence.text, "学习策略");
    assert_eq!(sentence.kind, CandidateKind::Sentence);
}

#[test]
fn keyboard_v_after_jqxy_matches_u_dictionary_keys() {
    let dictionary =
        Dictionary::parse("提取\tti qu\t9000\n去留\tqu liu\t8000\n学习\txue xi\t7000\n").unwrap();
    let mut engine = Engine::new(dictionary);

    engine.set_input("tiqv");
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "ti'qv");
    assert_eq!(query.candidates.items[0].text, "提取");
    assert_eq!(query.candidates.items[0].syllables, ["ti", "qu"]);

    engine.set_input("qvliu");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "去留");

    engine.set_input("xuexiqvliu");
    let sentence = &engine.query().unwrap().candidates.items[0];
    assert_eq!(sentence.text, "学习去留");
    assert_eq!(sentence.kind, CandidateKind::Sentence);
}

#[test]
fn empty_input_is_an_error() {
    assert_eq!(engine().query().unwrap_err(), ParseError::Empty);
}

#[test]
fn commit_consumes_only_the_candidate_syllables() {
    let mut engine = engine();
    engine.set_input("kaifazhe");
    let kaifa = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    assert_eq!(engine.commit(&kaifa), "开发");
    assert_eq!(engine.composition().text(), "zhe");

    engine.set_input("kaif");
    assert_eq!(engine.commit(&kaifa), "开发");
    assert!(engine.composition().is_empty());

    engine.set_input("xi'an");
    let xian = engine
        .query()
        .unwrap()
        .candidates
        .items
        .iter()
        .find(|c| c.text == "西安")
        .unwrap()
        .clone();
    engine.commit(&xian);
    assert!(engine.composition().is_empty());
}

#[test]
fn take_raw_returns_pinyin_and_clears() {
    let mut engine = engine();
    engine.set_input("kaifa");
    assert_eq!(engine.take_raw(), "kaifa");
    assert!(engine.composition().is_empty());
}

#[test]
fn choice_key_strips_separators_and_clamps() {
    assert_eq!(choice_key("kai'fa", 6), "kaifa");
    assert_eq!(choice_key("kai'fa", 3), "kai");
    assert_eq!(choice_key("ba", 10), "ba");
}

#[test]
fn sentence_conversion_leads_when_input_spans_several_words() {
    // 想開發：SAMPLE 裡沒有整詞，最優路徑是 想 + 開發
    let all = texts("xiangkaifa");
    assert_eq!(all[0], "想开发");
    let mut engine = engine();
    engine.set_input("xiangkaifa");
    let query = engine.query().unwrap();
    let sentence = query.candidates.items[0].clone();
    assert_eq!(sentence.kind, CandidateKind::Sentence);
    assert_eq!(sentence.syllables, ["xiang", "kai", "fa"]);
    assert_eq!(engine.commit(&sentence), "想开发");
    assert!(engine.composition().is_empty());

    // 整段本身是一個詞：不出整句
    let all = texts("kaifa");
    assert_eq!(all[0], "开发");
    assert_eq!(all.iter().filter(|t| *t == "开发").count(), 1);

    // 末尾只有一個字母時整句不算它：xiangkaif → 想開
    let all = texts("xiangkaif");
    assert_eq!(all[0], "想开");
}

#[test]
fn shortcuts_follow_the_first_local_candidate() {
    let all = texts("rq");
    let shortcut = all.iter().position(|t| t.ends_with('日')).unwrap();
    assert!(shortcut <= 1);
    assert!(all.iter().any(|t| t.contains('-')));

    let mut engine = engine();
    engine.set_input("xq");
    let query = engine.query().unwrap();
    let weekday = query
        .candidates
        .items
        .iter()
        .find(|c| c.kind == CandidateKind::Shortcut)
        .unwrap()
        .clone();
    assert!(weekday.text.starts_with("星期"));
    engine.commit(&weekday);
    assert!(engine.composition().is_empty());
}

#[test]
fn ng_alone_gives_en_interjection_first_in_full_pinyin_only() {
    let mut engine = engine();
    engine.set_input("ng");
    let query = engine.query().unwrap();
    let first = query.candidates.items[0].clone();
    assert_eq!(first.text, "嗯");
    engine.commit(&first);
    assert!(engine.composition().is_empty());

    engine.set_input("ngh");
    assert!(texts_of(&engine).iter().all(|t| t != "嗯"));

    engine.set_shuangpin(Some(Scheme::Xiaohe));
    engine.set_input("ng");
    assert!(texts_of(&engine).iter().all(|t| t != "嗯"));
}

#[test]
fn shift_letters_join_the_buffer_only_when_configured() {
    let dictionary = Dictionary::parse("C盘\tc pan\t8000\n磁盘\tci pan\t249\n").unwrap();
    let mut engine = Engine::new(dictionary);
    let type_cpan = |engine: &mut Engine| {
        // 中文模式下按住 Shift 敲 C，再打 pan
        engine.push('C');
        for c in "pan".chars() {
            engine.push(c);
        }
    };

    // 預設 `shift_letter = "passthrough"`：殼直接把大寫字母交給應用，這一路本來就不會走到；
    // 萬一走到也不該被當成拼音去匹配（所以「C盤」出不來）
    type_cpan(&mut engine);
    assert_ne!(engine.query().unwrap().candidates.items[0].text, "C盘");
    engine.clear();

    // 開了 compose：按小寫參與匹配，拼音行按敲的樣子顯示，回車原樣上屏時保留大寫
    engine.set_shift_letter_compose(true);
    type_cpan(&mut engine);
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "C盘");
    assert_eq!(query.marked_text(), "C'pan");
    assert_eq!(engine.take_raw(), "Cpan");
    assert!(engine.composition().is_empty());
}

#[test]
fn expression_mode_skips_pinyin_and_evaluates() {
    let mut engine = self::engine();
    assert!(!engine.expression_mode());
    engine.set_input("v1+2");
    assert!(engine.expression_mode());
    let query = engine.query().unwrap();
    assert_eq!(query.marked_text(), "v1+2");
    assert_eq!(query.marked_cursor(), 4);
    assert_eq!(query.candidates.items[0].text, "3");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Shortcut);
    assert_eq!(query.candidates.items[1].text, "1+2=3");
    let result = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&result), "3");
    assert!(engine.composition().is_empty());

    // 只有 v：候選為空但不報錯，preedit 照顯示
    engine.set_input("v");
    let query = engine.query().unwrap();
    assert!(query.candidates.items.is_empty());
    assert_eq!(query.marked_text(), "v");

    // v 開頭的英文詞仍能混輸
    let words = WordList::parse("very\tvery\t4800\n").unwrap();
    let mut engine = self::engine().with_english(words);
    engine.set_input("very");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "very");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::English);
}

/// 只認句首的 開發 與 開發 → 先 的假模型：讓兩詞路徑壓過整段的詞。
struct XianModel;

impl LanguageModel for XianModel {
    fn log_prob(&self, previous: Option<&str>, word: &str) -> Option<f64> {
        match (previous, word) {
            (None, "开发") => Some(-1.0),
            (Some("开发"), "先") => Some(-0.5),
            _ => None,
        }
    }
}

#[test]
fn a_word_spelling_the_sentence_keeps_its_rank_unless_its_reading_differs() {
    // 詞級排序裡 開發線 在 開發先 前面；整句轉換讀出的是 開發 + 先，與詞 開發先 同文本同讀音：不重複插，詞留在原位
    let sample = format!("{SAMPLE}开发线\tkai fa xian\t5000\n开发先\tkai fa xian\t1\n");
    let mut engine =
        Engine::new(Dictionary::parse(&sample).unwrap()).with_language_model(Box::new(XianModel));
    engine.set_input("kaifaxian");
    let all = texts_of(&engine);
    assert_eq!(&all[..2], ["开发线", "开发先"]);
    assert_eq!(all.iter().filter(|t| *t == "开发先").count(), 1);

    // 同文本的詞是按別的讀音（xiang，靠模糊音 an-ang 對上）收的：那條錯讀音的詞讓位，整句以正確讀音排最前
    let sample = format!("{SAMPLE}开发线\tkai fa xian\t5000\n开发先\tkai fa xiang\t1\n");
    let mut engine =
        Engine::new(Dictionary::parse(&sample).unwrap()).with_language_model(Box::new(XianModel));
    engine.set_fuzzy(FuzzyRules {
        an_ang: true,
        ..FuzzyRules::default()
    });
    engine.set_input("kaifaxian");
    let items = engine.query().unwrap().candidates.items;
    assert_eq!(items[0].text, "开发先");
    assert_eq!(items[0].kind, CandidateKind::Sentence);
    assert_eq!(items[0].syllables, ["kai", "fa", "xian"]);
    assert_eq!(items.iter().filter(|c| c.text == "开发先").count(), 1);
}

#[test]
fn option_backspace_deletes_a_syllable_and_command_backspace_deletes_to_the_start() {
    let mut engine = engine();
    // 全拼：刪最優切分的最後一個音節；`'` 連同前面的音節一起刪；切不動的尾巴整個刪
    engine.set_input("kaifaxian");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kaifa");
    engine.set_input("kai'fa'");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kai'");
    engine.set_input("kaifv");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kaif");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kai");
    // 游標停在中間：只動游標前的
    engine.set_input("kaifaxian");
    for _ in 0..4 {
        engine.move_cursor_left();
    }
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kaixian");
    assert_eq!(engine.composition().cursor(), 3);
    assert!(engine.delete_to_start());
    assert_eq!(engine.composition().text(), "xian");
    assert_eq!(engine.composition().cursor(), 0);
    assert!(!engine.delete_syllable_backward());
    assert!(!engine.delete_to_start());
    // 英文直輸段：字母一段一段刪，標點一次一個
    engine.set_input("hello,world");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "hello,");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "hello");
    // 雙拼：兩鍵一音節，落單的一鍵單刪
    let mut engine = xiaohe();
    engine.set_input("kdfah");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kdfa");
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "kd");
}

#[test]
fn option_arrows_move_the_cursor_by_syllable() {
    let mut engine = engine();
    // 游標在末尾發現前面錯了：⌥← 兩下跳到第二個音節後面，⌥⌫ 刪掉它，重敲，⌘→ 回末尾
    engine.set_input("kaifa'xian'xia");
    assert!(engine.move_cursor_syllable_left());
    assert_eq!(engine.composition().cursor(), "kaifa'xian'".len());
    assert!(engine.move_cursor_syllable_left());
    assert_eq!(engine.composition().cursor(), "kaifa'".len());
    assert!(engine.move_cursor_syllable_left());
    assert_eq!(engine.composition().cursor(), "kai".len());
    assert!(engine.delete_syllable_backward());
    assert_eq!(engine.composition().text(), "fa'xian'xia");
    engine.push('x');
    engine.push('i');
    assert_eq!(engine.composition().text(), "xifa'xian'xia");
    // ⌥→：從 xi| 起跳過一個音節到 xifa|，再跳先越過 `'` 再過一個音節
    assert!(engine.move_cursor_syllable_right());
    assert_eq!(engine.composition().cursor(), "xifa".len());
    assert!(engine.move_cursor_syllable_right());
    assert_eq!(engine.composition().cursor(), "xifa'xian".len());
    assert!(engine.move_cursor_syllable_right());
    assert_eq!(engine.composition().cursor(), "xifa'xian'xia".len());
    assert!(!engine.move_cursor_syllable_right());
    engine.move_cursor_home();
    assert!(!engine.move_cursor_syllable_left());
    // 直輸段按字母段跳
    engine.set_input("hello,world");
    assert!(engine.move_cursor_syllable_left());
    assert_eq!(engine.composition().cursor(), "hello,".len());
    engine.move_cursor_home();
    assert!(engine.move_cursor_syllable_right());
    assert_eq!(engine.composition().cursor(), "hello".len());
}
