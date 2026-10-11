//! 學習、撤銷、統計、日誌、釋義與詞彙。

use super::*;

#[test]
fn erasing_the_last_commit_and_choosing_another_word_retracts_its_learning() {
    let mut engine = engine().with_learner(Box::new(CountingLearner(HashMap::new())));
    let pick = |engine: &mut Engine, input: &str, text: &str| {
        engine.set_input(input);
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == text && c.kind == CandidateKind::Chinese)
            .unwrap();
        engine.commit(&candidate);
    };
    // 選了 開放，退兩格整個刪掉，重打 kaifa 選 開發：開放 的那次不算
    pick(&mut engine, "kaifa", "开放");
    assert_eq!(engine.learner().weight("开放"), 1);
    engine.note_backspace();
    engine.note_backspace();
    pick(&mut engine, "kaifa", "开发");
    assert_eq!(engine.learner().weight("开放"), 0);
    assert_eq!(engine.learner().choice_weight("kaifa", "开放"), 0);
    assert_eq!(engine.learner().weight("开发"), 1);
    // 只退了一格不算刪掉
    pick(&mut engine, "kaifa", "开放");
    engine.note_backspace();
    pick(&mut engine, "kaifa", "开发");
    assert_eq!(engine.learner().weight("开放"), 1);
    // 整個刪掉之後打的是別的拼音：不撤銷
    engine.note_backspace();
    engine.note_backspace();
    pick(&mut engine, "xian", "先");
    assert_eq!(engine.learner().weight("开发"), 2);
    // 刪得比記著的幾次上屏加起來（開放 先 開放 開發，7 個字）還多：在改別處，不撤銷
    pick(&mut engine, "kaifa", "开放");
    for _ in 0..8 {
        engine.note_backspace();
    }
    pick(&mut engine, "kaifa", "开发");
    assert_eq!(engine.learner().weight("开放"), 2);
}

#[test]
fn erasing_several_commits_and_retyping_retracts_the_wrong_one() {
    let mut engine = engine().with_learner(Box::new(CountingLearner(HashMap::new())));
    let pick = |engine: &mut Engine, input: &str, text: &str| {
        engine.set_input(input);
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == text && c.kind == CandidateKind::Chinese)
            .unwrap();
        engine.commit(&candidate);
    };
    // 打了 開放先 才發現 開放 錯了：三格刪光兩個詞，重打 kaifa 選 開發、再打 xian 選 先
    pick(&mut engine, "kaifa", "开放");
    pick(&mut engine, "xian", "先");
    for _ in 0..3 {
        engine.note_backspace();
    }
    pick(&mut engine, "kaifa", "开发");
    assert_eq!(engine.learner().weight("开放"), 0);
    assert_eq!(engine.learner().choice_weight("kaifa", "开放"), 0);
    // 先 重打後還是 先：不算選錯，計數照舊
    pick(&mut engine, "xian", "先");
    assert_eq!(engine.learner().weight("先"), 2);
    // 中間隔著標點與原樣上屏的英文也一樣：開放，gist先 全刪掉重打
    pick(&mut engine, "kaifa", "开放");
    assert_eq!(engine.punctuate(','), Some("，"));
    engine.set_input("gist");
    engine.take_raw();
    pick(&mut engine, "xian", "先");
    for _ in 0..8 {
        engine.note_backspace();
    }
    pick(&mut engine, "kaifa", "开发");
    assert_eq!(engine.learner().weight("开放"), 0);
    assert_eq!(engine.learner().weight("开发"), 2);
}

#[test]
fn splitting_a_buffer_into_a_word_and_a_sentence_forms_the_whole_phrase() {
    let shared = Arc::new(Mutex::new((Vec::new(), sentence::UserNgram::default())));
    let learner = WordLearner {
        shared: Arc::clone(&shared),
        ..WordLearner::default()
    };
    let mut engine = engine().with_learner(Box::new(learner));
    let mut phrase = String::new();
    // 每次都是先自選 想，剩下的 kaifaxian 用整句候選上屏
    for round in 1..=2 {
        engine.set_input("xiangkaifaxian");
        let xiang = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == "想" && c.kind == CandidateKind::Chinese)
            .unwrap();
        engine.commit(&xiang);
        assert_eq!(engine.composition().text(), "kaifaxian");
        let rest = engine.query().unwrap().candidates.items[0].clone();
        assert_eq!(rest.kind, CandidateKind::Sentence, "round {round}");
        let head = rest.text.strip_suffix('先').unwrap().to_owned();
        engine.commit(&rest);
        assert!(engine.composition().is_empty());
        phrase = format!("想{}", rest.text);
        // 接縫處（想 → 開X）按自選記雙份，整句內部的接續記一份；整段拼音 → 合成詞 記一次選擇
        let learned = shared.lock().unwrap();
        assert_eq!(
            learned.1.pair(Some("想"), &head),
            round * EXPLICIT_TRANSITION_WEIGHT
        );
        assert_eq!(learned.1.pair(Some(&head), "先"), round);
        drop(learned);
        assert_eq!(
            engine.learner().choice_weight("xiangkaifaxian", &phrase),
            round
        );
        // 第二次：整段合成 想開X先；接縫處不造兩字詞
        let words = shared.lock().unwrap().0.clone();
        if round == 1 {
            assert!(words.is_empty(), "{words:?}");
        } else {
            assert_eq!(words, [phrase.clone()]);
        }
        engine.note_passthrough('\n');
    }
    // 第三次整段打出來：合成詞直接排第一
    engine.set_input("xiangkaifaxian");
    assert_eq!(texts_of(&engine)[0], phrase);
}

#[test]
fn commit_feeds_learner_and_reorders() {
    let mut engine = engine().with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_input("kaif");
    let query = engine.query().unwrap();
    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .unwrap()
        .clone();
    assert_eq!(engine.commit(&kaifa), "开发");
    assert!(engine.composition().is_empty());

    // 同一輸入串再打：選過的 開發 壓過詞頻更高的 開放
    engine.set_input("kaif");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发");
    // 換個輸入串（`kai'fa`，分隔符不計）也算同一串；`kaifang` 不算
    engine.set_input("kai'f");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发");
    engine.set_input("kaifazhe");
    let items = engine.query().unwrap().candidates.items;
    let kaifa = items.iter().find(|c| c.text == "开发").unwrap().clone();
    // 從長輸入裡選字首詞：按消耗掉的那段（`kaifa`）記
    engine.commit(&kaifa);
    engine.set_input("kaifa");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发");
}

#[test]
fn missing_glosses_are_requested_on_commit_and_learned_when_they_arrive() {
    let filler = MemoryFiller::default();
    let requested = filler.requested.clone();
    let ready = filler.ready.clone();
    let mut engine = engine()
        .with_translator(Box::new(LearningTranslator::default()))
        .with_gloss_filler(Box::new(filler));
    // 開發 有釋義，不問；開放 沒有，上屏後問
    for input in ["kaifa", "kaifang"] {
        engine.set_input(input);
        let query = engine.query().unwrap();
        let candidate = query.candidates.items[0].clone();
        engine.commit(&candidate);
    }
    assert_eq!(*requested.lock().unwrap(), vec!["开放".to_owned()]);
    assert_eq!(engine.poll_glosses(), 0);
    let translation = Translation::new(
        Language::English,
        vec![Sense {
            part_of_speech: Some(PartOfSpeech::Adjective),
            text: "open".into(),
            reading: None,
            fresh: false,
        }],
    );
    ready.lock().unwrap().push(FilledGloss {
        word: "开放".into(),
        translation: translation.clone(),
    });
    // 語言對不上的丟掉
    ready.lock().unwrap().push(FilledGloss {
        word: "开放".into(),
        translation: Translation::new(Language::Japanese, Vec::new()),
    });
    assert_eq!(engine.poll_glosses(), 1);
    engine.set_input("kaifang");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    assert_eq!(
        query.candidates.items[0]
            .translation
            .as_ref()
            .unwrap()
            .senses()[0]
            .text,
        "open"
    );
    // 沒有釋義兜底時不問
    let mut plain = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_translator(Box::new(LearningTranslator::default()));
    plain.set_input("kaifang");
    let candidate = plain.query().unwrap().candidates.items[0].clone();
    plain.commit(&candidate);
}

#[test]
fn fresh_translations_are_marked_until_seen_enough_times_at_commit() {
    let book = Arc::new(Mutex::new(HashMap::new()));
    let mut engine = engine()
        .with_translator(Box::new(FixedTranslator))
        .with_vocabulary_tracker(Box::new(MemoryVocabulary(book.clone())));
    let key = (Language::English, "develop".to_owned());
    for round in 0..FRESH_UNTIL {
        engine.set_input("kaifa");
        let mut query = engine.query().unwrap();
        engine.annotate(&mut query.candidates);
        let kaifa = query.candidates.items[0].clone();
        assert_eq!(kaifa.text, "开发");
        assert!(
            kaifa.translation.as_ref().unwrap().senses()[0].fresh,
            "第 {round} 轮还该标生词"
        );
        // 畫出來只是記下當前頁，上屏那一刻才算看到
        engine.note_displayed(query.candidates.items.iter());
        let before = if round == 0 {
            None
        } else {
            Some((round, round, 0))
        };
        assert_eq!(book.lock().unwrap().get(&key).copied(), before);
        engine.commit(&kaifa);
        assert_eq!(
            book.lock().unwrap().get(&key).copied(),
            Some((round + 1, round + 1, 0))
        );
    }
    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let kaifa = query.candidates.items[0].clone();
    assert!(!kaifa.translation.as_ref().unwrap().senses()[0].fresh);
    // 直接打出譯詞算「用過」
    engine.note_displayed(query.candidates.items.iter());
    assert_eq!(
        engine.commit_translation(&kaifa, 0).as_deref(),
        Some("develop")
    );
    assert_eq!(
        book.lock().unwrap().get(&key).copied(),
        Some((FRESH_UNTIL + 1, FRESH_UNTIL + 1, 1))
    );
    // 沒有詞彙記錄時不標生詞
    let mut plain =
        Engine::new(Dictionary::parse(SAMPLE).unwrap()).with_translator(Box::new(FixedTranslator));
    plain.set_input("kaifa");
    let mut query = plain.query().unwrap();
    plain.annotate(&mut query.candidates);
    assert!(
        !query.candidates.items[0]
            .translation
            .as_ref()
            .unwrap()
            .senses()[0]
            .fresh
    );
}

#[test]
fn input_log_records_commits_with_their_context_and_retractions() {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let mut engine = engine().with_input_logger(Box::new(MemoryLogger(entries.clone())));
    engine.set_input("kaifazhe");
    let query = engine.query().unwrap();
    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .cloned()
        .unwrap();
    engine.commit(&kaifa);
    {
        let entries = entries.lock().unwrap();
        let InputLogEntry::Commit(commit) = &entries[0] else {
            panic!("expected a commit");
        };
        assert_eq!(commit.id, 1);
        assert_eq!(commit.scope, "kaifazhe");
        assert_eq!(commit.keys, "kaifa");
        assert_eq!(commit.pinyin, "kai'fa'zhe");
        assert_eq!(commit.text, "开发");
        assert_eq!(commit.source, InputSource::Word);
        assert_eq!(commit.index, Some(1));
        assert_eq!(commit.top[0], "开发者");
        assert!(!commit.corrected);
    }
    // 剩下的 zhe 回車原樣上屏
    assert_eq!(engine.take_raw(), "zhe");
    // 整個退格刪掉 開發 再重打同一段拼音換選 開放：記一條撤銷
    engine.set_input("kaifa");
    engine.query().unwrap();
    engine.commit(&kaifa);
    for _ in 0..2 {
        engine.note_backspace();
    }
    engine.set_input("kaifa");
    let query = engine.query().unwrap();
    let kaifang = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开放")
        .cloned()
        .unwrap();
    engine.commit(&kaifang);
    let entries = entries.lock().unwrap();
    let kinds: Vec<&str> = entries
        .iter()
        .map(|e| match e {
            InputLogEntry::Commit(c) => match c.source {
                InputSource::Raw => "raw",
                _ => "commit",
            },
            InputLogEntry::Retract { .. } => "retract",
            _ => "other",
        })
        .collect();
    assert_eq!(kinds, ["commit", "raw", "commit", "retract", "commit"]);
    assert!(matches!(
        &entries[3],
        InputLogEntry::Retract { of: 3, text, chosen } if text == "开发" && chosen == "开放"
    ));
}

#[test]
fn word_ranking_follows_the_previous_committed_word() {
    let dictionary =
        Dictionary::parse("把\tba\t3000000\n吧\tba\t2000000\n做了\tzuo le\t5000\n").unwrap();
    let mut engine = Engine::new(dictionary).with_language_model(Box::new(BaModel));
    let first = |engine: &mut Engine| engine.query().unwrap().candidates.items[0].text.clone();
    // 句首：模型說 把
    engine.set_input("ba");
    assert_eq!(first(&mut engine), "把");
    // 做了 上屏之後再打 ba：吧
    engine.set_input("zuole");
    let done = engine.query().unwrap().candidates.items[0].clone();
    engine.commit(&done);
    engine.set_input("ba");
    assert_eq!(first(&mut engine), "吧");
    // 標點斷句後回到句首
    engine.punctuate('。');
    engine.set_input("ba");
    assert_eq!(first(&mut engine), "把");
}

#[test]
fn erasing_a_committed_sentence_retracts_its_transitions() {
    let shared = Arc::new(Mutex::new((Vec::new(), sentence::UserNgram::default())));
    let learner = WordLearner {
        shared: Arc::clone(&shared),
        ..WordLearner::default()
    };
    let mut engine = engine().with_learner(Box::new(learner));
    engine.set_input("xiangkaifa");
    let sentence = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(sentence.kind, CandidateKind::Sentence);
    engine.commit(&sentence);
    assert_eq!(shared.lock().unwrap().1.pair(Some("想"), "开发"), 1);
    // 想開發 三個字全刪掉，重打字首 xiang 選 想：整句路徑的兩條轉移退回，想 按點選記雙份
    for _ in 0..3 {
        engine.note_backspace();
    }
    engine.set_input("xiangkaifa");
    let xiang = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "想" && c.kind == CandidateKind::Chinese)
        .unwrap();
    engine.commit(&xiang);
    let ngram = &shared.lock().unwrap().1;
    assert_eq!(ngram.pair(Some("想"), "开发"), 0);
    assert_eq!(ngram.pair(None, "想"), EXPLICIT_TRANSITION_WEIGHT);
}

#[test]
fn commits_feed_the_personal_bigram_and_form_words() {
    let shared = Arc::new(Mutex::new((Vec::new(), sentence::UserNgram::default())));
    let learner = WordLearner {
        shared: Arc::clone(&shared),
        ..WordLearner::default()
    };
    let mut engine = engine().with_learner(Box::new(learner));

    // 整句上屏：按路徑上的詞記轉移，第一個詞在句首
    engine.set_input("xiangkaifa");
    let sentence = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(sentence.kind, CandidateKind::Sentence);
    engine.commit(&sentence);
    {
        let ngram = &shared.lock().unwrap().1;
        assert_eq!(ngram.pair(None, "想"), 1);
        assert_eq!(ngram.pair(Some("想"), "开发"), 1);
    }

    // 標點斷句；之後連續從同一段拼音裡選 開發 + 先：第一次只記轉移，第二次自動造詞 開發先
    engine.punctuate('，');
    let select = |engine: &mut Engine, text: &str| {
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == text && c.kind == CandidateKind::Chinese)
            .unwrap();
        engine.commit(&candidate);
    };
    for round in 1..=2 {
        engine.set_input("kaifaxian");
        select(&mut engine, "开发");
        assert_eq!(engine.composition().text(), "xian");
        select(&mut engine, "先");
        assert!(engine.composition().is_empty());
        // 使用者自己點選的詞，轉移按 EXPLICIT_TRANSITION_WEIGHT 份記
        let learned = shared.lock().unwrap();
        assert_eq!(
            learned.1.pair(None, "开发"),
            round * EXPLICIT_TRANSITION_WEIGHT
        );
        assert_eq!(
            learned.1.pair(Some("开发"), "先"),
            round * EXPLICIT_TRANSITION_WEIGHT
        );
        assert_eq!(learned.0.len(), usize::from(round == 2), "round {round}");
        engine.note_passthrough('\n');
    }
    assert_eq!(shared.lock().unwrap().0, ["开发先"]);

    // 分兩段打的要三次：咖啡 + 開 記兩次不造詞，第三次造
    for round in 1..=3 {
        engine.set_input("kafei");
        select(&mut engine, "咖啡");
        engine.set_input("kai");
        select(&mut engine, "开");
        assert_eq!(
            shared.lock().unwrap().1.pair(Some("咖啡"), "开"),
            round * EXPLICIT_TRANSITION_WEIGHT
        );
        assert_eq!(shared.lock().unwrap().0.len(), 1 + usize::from(round == 3));
        engine.punctuate('。');
    }
    assert_eq!(shared.lock().unwrap().0, ["开发先", "咖啡开"]);
}

#[test]
fn committing_the_translation_learns_the_word_and_returns_the_gloss() {
    struct KaifaTranslator;
    impl Translator for KaifaTranslator {
        fn language(&self) -> Language {
            Language::Japanese
        }
        fn translate(&self, text: &str) -> Option<Translation> {
            (text == "开发").then(|| {
                Translation::new(
                    Language::Japanese,
                    vec![Sense {
                        part_of_speech: Some(PartOfSpeech::Verb),
                        text: "開発する".into(),
                        reading: Some("かいはつする".into()),
                        fresh: false,
                    }],
                )
            })
        }
    }
    let mut engine = engine()
        .with_translator(Box::new(KaifaTranslator))
        .with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_input("kaifazhe");
    let mut query = engine.query().unwrap();
    engine.annotate(&mut query.candidates);
    let kaifa = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "开发")
        .cloned()
        .unwrap();
    assert_eq!(
        engine.commit_translation(&kaifa, 0).as_deref(),
        Some("開発する")
    );
    // 拼音按候選消耗，剩下的接著組句；這個詞記了學習
    assert_eq!(engine.composition().text(), "zhe");
    assert_eq!(engine.learner().weight("开发"), 1);
    // 沒有譯文的候選不動
    let zhe = Candidate {
        text: "者".into(),
        kind: CandidateKind::Chinese,
        syllables: vec!["zhe".into()],
        reading: None,
        translation: None,
        aux_code: None,
    };
    assert_eq!(engine.commit_translation(&zhe, 0), None);
    assert_eq!(engine.composition().text(), "zhe");
}

#[test]
fn annotate_fills_only_known_words() {
    let mut engine = engine().with_translator(Box::new(FixedTranslator));
    engine.set_input("kaifa");
    let mut query = engine.query().unwrap();
    assert!(
        query
            .candidates
            .items
            .iter()
            .all(|c| c.translation.is_none())
    );
    let report = engine.annotate(&mut query.candidates);
    assert_eq!(report.hits, 1);
    assert_eq!(report.total, 5);
    assert_eq!(
        query.candidates.items[0]
            .translation
            .as_ref()
            .unwrap()
            .senses()[0]
            .text,
        "develop"
    );
}

#[test]
fn extra_dictionaries_join_lookup_and_sentences() {
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap());
    engine.set_input("zhangtao");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.text != "账套")
    );
    let extra = Dictionary::parse("账套\tzhang tao\t500\n").unwrap();
    engine.set_extra_dictionaries(vec![extra]);
    engine.set_input("zhangtao");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "账套");
    // 整句詞圖也能用上附加詞庫裡的詞
    engine.set_input("kaifazhangtao");
    let sentence = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.kind == CandidateKind::Sentence)
        .unwrap();
    assert_eq!(sentence.text, "开发账套");
    engine.set_extra_dictionaries(Vec::new());
    engine.set_input("zhangtao");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.text != "账套")
    );
}

/// 建一個帶記憶體日誌的引擎，返回引擎與條目列表。
fn logged_engine() -> (Engine, Arc<Mutex<Vec<InputLogEntry>>>) {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let engine = engine().with_input_logger(Box::new(MemoryLogger(entries.clone())));
    (engine, entries)
}

fn commit_first(engine: &mut Engine) -> String {
    let query = engine.query().unwrap();
    let first = query.candidates.items[0].clone();
    engine.commit(&first)
}

#[test]
fn empty_take_raw_is_not_logged() {
    let (mut engine, entries) = logged_engine();
    assert_eq!(engine.take_raw(), "");
    assert!(entries.lock().unwrap().is_empty());
}

#[test]
fn input_log_records_a_retype_when_keys_change_after_backspace_in_composition() {
    let (mut engine, entries) = logged_engine();
    for c in "kaifs".chars() {
        engine.push(c);
    }
    engine.backspace();
    engine.push('a');
    // 再刪一次不換快照：記的是第一次退格前的串
    engine.backspace();
    engine.push('a');
    assert_eq!(commit_first(&mut engine), "开发");
    let entries = entries.lock().unwrap();
    let InputLogEntry::Commit(commit) = &entries[0] else {
        panic!("expected a commit");
    };
    assert_eq!(commit.keys, "kaifa");
    assert_eq!(commit.pages, 0);
    assert!(!commit.rescored);
    assert!(matches!(
        &entries[1],
        InputLogEntry::Retype { before, after, of: 1 } if before == "kaifs" && after == "kaifa"
    ));
    assert_eq!(entries.len(), 2);
}

#[test]
fn backspace_that_restores_the_same_keys_is_not_a_retype() {
    let (mut engine, entries) = logged_engine();
    for c in "kaifa".chars() {
        engine.push(c);
    }
    engine.backspace();
    engine.push('a');
    commit_first(&mut engine);
    assert_eq!(entries.lock().unwrap().len(), 1);
}

#[test]
fn input_log_records_a_retype_across_commits_when_similar_keys_are_retyped() {
    let (mut engine, entries) = logged_engine();
    engine.set_input("kaifa");
    commit_first(&mut engine);
    for _ in 0..2 {
        engine.note_backspace();
    }
    engine.set_input("kaifan");
    commit_first(&mut engine);
    let entries = entries.lock().unwrap();
    assert!(matches!(
        &entries[1],
        InputLogEntry::Retype { before, after, of: 1 } if before == "kaifa" && after == "kaifan"
    ));
    assert!(matches!(&entries[2], InputLogEntry::Commit(_)));
}

#[test]
fn input_log_flushes_passthrough_before_the_next_commit_and_marks_breaks() {
    let (mut engine, entries) = logged_engine();
    engine.set_application(Some("com.apple.TextEdit".into()));
    // 還沒上屏過：失焦不記 break
    engine.break_chain();
    engine.note_passthrough(',');
    engine.note_passthrough(' ');
    engine.set_input("kaifa");
    commit_first(&mut engine);
    engine.note_passthrough('.');
    engine.break_chain();
    engine.break_chain();
    let entries = entries.lock().unwrap();
    assert!(matches!(&entries[0], InputLogEntry::Passthrough { text } if text == ", "));
    let InputLogEntry::Commit(commit) = &entries[1] else {
        panic!("expected a commit");
    };
    assert_eq!(commit.app.as_deref(), Some("com.apple.TextEdit"));
    assert!(matches!(&entries[2], InputLogEntry::Passthrough { text } if text == "."));
    assert!(matches!(
        &entries[3],
        InputLogEntry::Break { app: Some(app) } if app == "com.apple.TextEdit"
    ));
    assert_eq!(entries.len(), 4);
}

#[test]
fn input_log_records_the_session_and_page_turns() {
    let (mut engine, entries) = logged_engine();
    engine.log_session("0.1.1", "cli");
    engine.set_input("kaifa");
    engine.note_page_turn();
    engine.note_page_turn();
    commit_first(&mut engine);
    let entries = entries.lock().unwrap();
    assert!(matches!(
        &entries[0],
        InputLogEntry::Session { v: INPUT_LOG_VERSION, version, platform, model: false, scheme }
            if version == "0.1.1" && platform == "cli" && scheme.is_empty()
    ));
    let InputLogEntry::Commit(commit) = &entries[1] else {
        panic!("expected a commit");
    };
    assert_eq!(commit.pages, 2);
}
