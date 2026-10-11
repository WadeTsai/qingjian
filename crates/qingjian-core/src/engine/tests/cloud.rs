//! 雲聯想 / 問字 / 翻譯請求。

use super::*;

#[test]
fn question_mode_asks_the_cloud_and_shows_answers_unvalidated() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let answer = CloudWord {
        text: "森".into(),
        syllables: Vec::new(),
        reading: Some("sēn".into()),
    };
    let restated = CloudWord {
        text: "木木木是什么字".into(),
        syllables: Vec::new(),
        reading: None,
    };
    let predictor = EchoPredictor {
        submitted: submitted.clone(),
        replies: vec![Prediction {
            sequence: 1,
            words: vec![restated, answer],
            sentence: None,
        }],
        sentence: true,
    };
    let mut engine = self::engine().with_predictor(Box::new(predictor));
    // 預設 `?` 不是入口：開了開關才進問字
    assert!(!engine.takes_question_mark());
    engine.set_mode_keys(ModeKeys {
        question_mark: true,
        ..ModeKeys::default()
    });
    assert!(engine.takes_question_mark());
    engine.push('?');
    assert!(engine.bare_question());
    assert!(engine.question_mode());
    for c in "mumumu".chars() {
        engine.push(c);
    }
    assert!(!engine.bare_question());
    let query = engine.query().unwrap();
    assert!(query.candidates.items.is_empty());
    assert_eq!(query.marked_text(), "?mu'mu'mu");
    assert_eq!(query.marked_cursor(), 9);

    assert_eq!(engine.request_prediction(None, &[]), Some(1));
    let request = submitted.borrow()[0].clone();
    assert_eq!(request.kind, PredictionKind::Question);
    assert_eq!(request.pinyin, "mu'mu'mu");
    assert_eq!(request.letters, "mumumu");
    assert!(!request.want_sentence);

    // 答案的拼音與敲的字母對不上，但問字模式不校驗；複述問題的「答案」剔掉
    let prediction = engine.poll_prediction().unwrap();
    assert_eq!(prediction.words.len(), 1);
    assert_eq!(prediction.words[0].text, "森");
    let answer = prediction
        .words
        .into_iter()
        .next()
        .unwrap()
        .into_candidate();
    assert_eq!(answer.reading.as_deref(), Some("sēn"));
    assert_eq!(engine.commit(&answer), "森");
    assert!(engine.composition().is_empty());
}

#[test]
fn prediction_request_trims_context_and_only_fires_while_composing() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut engine = engine().with_predictor(Box::new(EchoPredictor {
        submitted: submitted.clone(),
        replies: Vec::new(),
        sentence: true,
    }));
    engine.set_input("kaifa");
    let query = engine.query().unwrap();
    let surrounding = SurroundingText {
        before: "我们今天一起来".into(),
        after: "吧，好不好".into(),
    };
    let sequence = engine.request_prediction(Some(surrounding), &query.candidates.items);
    assert_eq!(sequence, Some(1));
    let request = submitted.borrow()[0].clone();
    assert_eq!(request.before, "天一起来");
    assert_eq!(request.after, "吧，");
    assert_eq!(request.pinyin, "kai'fa");
    assert_eq!(request.letters, "kaifa");
    assert_eq!(request.syllables, 2);
    assert_eq!(request.candidates[0], "开发");
    assert_eq!(request.max_items, 2);
    assert!(request.want_sentence);

    // 拼音太短不發
    engine.set_input("k");
    assert_eq!(engine.request_prediction(None, &[]), None);
    engine.set_input("kaifa");
    // 沒有應用上下文也照發：詞候選和整句都只靠拼音；本地歷史不進請求
    let kaifa = query.candidates.items[0].clone();
    engine.commit(&kaifa);
    engine.punctuate('.');
    engine.set_input("kaifa");
    assert_eq!(engine.request_prediction(None, &[]), Some(3));
    let request = submitted.borrow()[1].clone();
    assert!(request.before.is_empty());
    assert!(request.want_sentence);
    // 上屏之後不聯想
    engine.clear();
    assert_eq!(engine.request_prediction(None, &[]), None);
}

/// 簡拼（半數以上音節是縮寫）只問整句補全：模型按聲母湊出來的詞大多是生造詞（複合語氣、符號對映）。
#[test]
fn abbreviated_input_only_asks_for_the_sentence() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut engine = engine().with_predictor(Box::new(EchoPredictor {
        submitted: submitted.clone(),
        replies: Vec::new(),
        sentence: true,
    }));
    engine.set_input("zhsh");
    let query = engine.query().unwrap();
    assert_eq!(
        engine.request_prediction(None, &query.candidates.items),
        Some(1)
    );
    let request = submitted.borrow()[0].clone();
    assert_eq!(request.letters, "zhsh");
    assert_eq!(request.max_items, 0, "简拼不该要词");
    assert!(request.want_sentence, "整句补全照常要");

    // 完整拼音照常要詞
    engine.set_input("zhongshi");
    let query = engine.query().unwrap();
    assert_eq!(
        engine.request_prediction(None, &query.candidates.items),
        Some(2)
    );
    let request = submitted.borrow()[1].clone();
    assert_eq!(request.max_items, 2);
    assert!(request.want_sentence);
}

#[test]
fn cursor_in_the_middle_scopes_candidates_and_prediction_to_the_left_part() {
    let mut engine = engine();
    engine.set_input("kaifazhe");
    for _ in 0..3 {
        engine.move_cursor_left();
    }
    // kaifa|zhe：候選只看 kaifa（與單獨打 kaifa 一樣），zhe 只畫出來
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "开发");
    assert_eq!(query.marked_text(), "kai'fa'zhe");
    assert_eq!(query.marked_cursor(), 6);

    // 上屏 開發 後剩 zhe，游標落到末尾，接著打就是往後加
    let kaifa = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&kaifa), "开发");
    assert_eq!(engine.composition().text(), "zhe");
    assert_eq!(engine.composition().cursor(), 3);

    // 游標在開頭時按整段算
    engine.set_input("kaifa");
    engine.move_cursor_home();
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发");
}

#[test]
fn prediction_request_uses_the_scope_only() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let predictor = EchoPredictor {
        submitted: submitted.clone(),
        replies: Vec::new(),
        sentence: false,
    };
    let mut engine = engine().with_predictor(Box::new(predictor));
    engine.set_input("kaifazhe");
    for _ in 0..3 {
        engine.move_cursor_left();
    }
    assert_eq!(engine.request_prediction(None, &[]), Some(1));
    let request = submitted.borrow()[0].clone();
    assert_eq!(request.pinyin, "kai'fa");
    assert_eq!(request.letters, "kaifa");
    assert_eq!(request.syllables, 2);
}

#[test]
fn stale_predictions_are_dropped_and_accept_clears_composition() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut engine = engine().with_predictor(Box::new(EchoPredictor {
        submitted,
        sentence: false,
        replies: vec![
            Prediction {
                sequence: 2,
                words: vec![
                    cloud("开花", &["kai", "hua"]),
                    cloud("凯发", &["kai", "fa"]),
                ],
                sentence: Some("开发输入法".into()),
            },
            Prediction {
                sequence: 1,
                words: Vec::new(),
                sentence: Some("旧结果".into()),
            },
        ],
    }));
    engine.set_input("kaifa");
    engine.request_prediction(None, &[]);
    engine.request_prediction(None, &[]);
    let prediction = engine.poll_prediction().unwrap();
    assert_eq!(prediction.sequence, 2);
    assert_eq!(prediction.sentence.as_deref(), Some("开发输入法"));
    // 開花 的拼音對不上 kaifa，被過濾
    assert_eq!(prediction.words, [cloud("凯发", &["kai", "fa"])]);
    assert_eq!(engine.poll_prediction(), None);

    // 取消後連當前序號的結果也不要
    engine.request_prediction(None, &[]);
    engine.cancel_prediction();
    assert_eq!(engine.poll_prediction(), None);

    assert_eq!(engine.accept_prediction("开发输入法"), "开发输入法");
    assert!(engine.composition().is_empty());
    assert_eq!(engine.history().text(), "开发输入法");
}

#[test]
fn cloud_words_tolerate_typos_but_not_unrelated_words() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut engine = engine().with_predictor(Box::new(EchoPredictor {
        submitted,
        sentence: false,
        replies: vec![Prediction {
            sequence: 1,
            words: vec![
                cloud("这个东西吗", &["zhe", "ge", "dong", "xi", "ma"]),
                cloud("知道", &["zhi", "dao"]),
            ],
            sentence: None,
        }],
    }));
    engine.set_input("zhgdoima");
    engine.request_prediction(None, &[]);
    let prediction = engine.poll_prediction().unwrap();
    assert_eq!(
        prediction.words,
        [cloud("这个东西吗", &["zhe", "ge", "dong", "xi", "ma"])]
    );
    // 雲端詞上屏吃掉整段（按音節對不上的）拼音
    let word = Candidate {
        text: "这个东西吗".into(),
        kind: CandidateKind::Cloud,
        syllables: prediction.words[0].syllables.clone(),
        reading: None,
        translation: None,
        aux_code: None,
    };
    assert_eq!(engine.commit(&word), "这个东西吗");
    assert!(engine.composition().is_empty());
}

#[test]
fn cloud_words_are_validated_against_abbreviated_pinyin() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut engine = engine().with_predictor(Box::new(EchoPredictor {
        submitted,
        sentence: false,
        replies: vec![Prediction {
            sequence: 1,
            words: vec![
                cloud("账套", &["zhang", "tao"]),
                cloud("张涛涛", &["zhang", "tao", "tao"]),
                cloud("知道", &["zhi", "dao"]),
                cloud("不是音节", &["zx", "tq", "a", "b"]),
            ],
            sentence: None,
        }],
    }));
    engine.set_input("zt");
    engine.request_prediction(None, &[]);
    let prediction = engine.poll_prediction().unwrap();
    assert_eq!(prediction.words, [cloud("账套", &["zhang", "tao"])]);
}

#[test]
fn accepted_sentence_completion_feeds_the_personal_ngram_and_can_be_retracted() {
    let shared = Arc::new(Mutex::new((Vec::new(), sentence::UserNgram::default())));
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_language_model(Box::new(SentenceModel))
        .with_learner(Box::new(WordLearner {
            shared: shared.clone(),
            ..WordLearner::default()
        }));
    engine.set_input("kaifa");
    assert_eq!(
        engine.accept_prediction("开发输入法很好用。"),
        "开发输入法很好用。"
    );
    {
        let ngram = &shared.lock().unwrap().1;
        assert_eq!(ngram.pair(None, "开发"), 1);
        assert_eq!(ngram.pair(Some("开发"), "输入法"), 1);
        assert_eq!(ngram.pair(Some("输入法"), "很"), 1);
        assert_eq!(ngram.pair(Some("很"), "好用"), 1);
    }
    // 句尾是句號：下一個詞按句首記
    assert_eq!(engine.chain.previous(), None);

    // 整句退格刪光、同一段拼音重新選詞：撤銷這句記的轉移
    for _ in 0.."开发输入法很好用。".chars().count() {
        engine.note_backspace();
    }
    engine.set_input("kaifa");
    let query = engine.query().unwrap();
    let first = query.candidates.items[0].clone();
    engine.commit(&first);
    let ngram = &shared.lock().unwrap().1;
    assert_eq!(ngram.pair(Some("开发"), "输入法"), 0);
    assert_eq!(ngram.pair(Some("很"), "好用"), 0);
}

#[test]
fn question_key_answers_code_points_locally_and_keeps_question_mark_alias() {
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut engine = engine().with_predictor(Box::new(EchoPredictor {
        submitted: submitted.clone(),
        replies: Vec::new(),
        sentence: false,
    }));
    engine.set_input("u4e00");
    assert!(engine.question_mode());
    assert!(engine.unicode_entry());
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "一");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Shortcut);
    assert_eq!(query.tail, "u4e00");
    // 碼點本地就答，不問雲端
    assert!(
        engine
            .request_prediction(None, &query.candidates.items)
            .is_none()
    );
    assert!(submitted.borrow().is_empty());

    engine.set_input("usangemu");
    assert!(engine.question_mode());
    assert!(!engine.unicode_entry());
    let query = engine.query().unwrap();
    assert!(query.candidates.items.is_empty());
    let body = query.tail.strip_prefix('u').unwrap().to_owned();
    assert!(!body.is_empty());

    // `?` 預設不是入口：`?sangemu` 是英文直輸段而不是問題
    engine.set_input("?sangemu");
    assert!(!engine.question_mode() && engine.raw_mode());
    // 開了開關才是別名：同一個問題、同樣的切分，只是字首不同
    engine.set_mode_keys(ModeKeys {
        question_mark: true,
        ..ModeKeys::default()
    });
    assert!(engine.question_mode());
    assert_eq!(engine.query().unwrap().tail, format!("?{body}"));

    // 換成 i 問字、v 表示式後 u 就是普通字母
    engine.set_mode_keys(ModeKeys {
        expression: 'v',
        question: 'i',
        question_mark: false,
    });
    engine.set_input("u4e00");
    assert!(!engine.question_mode());
    engine.set_input("i4e00");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "一");
    // 非法組合退回預設
    engine.set_mode_keys(ModeKeys {
        expression: 'u',
        question: 'u',
        question_mark: false,
    });
    assert_eq!(engine.mode_keys(), ModeKeys::default());
}

#[test]
fn committing_a_cloud_word_learns_it_and_it_ranks_first_next_time() {
    let mut engine = engine().with_learner(Box::new(WordLearner::default()));
    engine.set_input("zt");
    assert!(engine.query().unwrap().candidates.items.is_empty());
    let word = Candidate {
        text: "账套".into(),
        kind: CandidateKind::Cloud,
        syllables: vec!["zhang".into(), "tao".into()],
        reading: None,
        translation: None,
        aux_code: None,
    };
    assert_eq!(engine.commit(&word), "账套");
    assert!(engine.composition().is_empty());

    engine.set_input("zhangtao");
    let all = texts_of(&engine);
    assert_eq!(all[0], "账套");
    engine.set_input("zt");
    assert_eq!(texts_of(&engine)[0], "账套");

    // 詞庫裡已有的雲端詞不重複記
    engine.set_input("kaifa");
    let mut kaifa = engine.query().unwrap().candidates.items[0].clone();
    kaifa.kind = CandidateKind::Cloud;
    engine.commit(&kaifa);
    engine.set_input("kaifa");
    assert_eq!(texts_of(&engine).iter().filter(|t| *t == "开发").count(), 1);
}

#[test]
fn traditional_mode_preserves_original_text_across_queries() {
    let mut engine = engine()
        .with_predictor(Box::new(EchoPredictor {
            submitted: std::rc::Rc::new(std::cell::RefCell::new(Vec::new())),
            sentence: false,
            replies: vec![Prediction {
                sequence: 1,
                words: vec![cloud("凯发", &["kai", "fa"])],
                sentence: None,
            }],
        }))
        .with_learner(Box::new(WordLearner::default()));

    engine.set_traditional_mode(true);
    engine.set_input("kaifa");
    engine.request_prediction(None, &[]);
    let prediction = engine.poll_prediction().unwrap();
    let cloud_text = prediction.words[0].text.clone();

    engine.query().unwrap(); // 第二次 query() 不應清空雲端詞的對映

    let word = Candidate {
        text: cloud_text,
        kind: CandidateKind::Cloud,
        syllables: vec!["kai".into(), "fa".into()],
        reading: None,
        translation: None,
        aux_code: None,
    };
    assert_eq!(engine.commit(&word), "凱發");
    // 檢查詞庫裡學到的是簡體「凱發」
    assert!(engine.learner().weight("凯发") > 0);
    assert_eq!(engine.learner().weight("凱發"), 0);
}

#[test]
fn cloud_words_are_learned_with_the_typed_reading_when_it_fits() {
    let mut engine = engine().with_learner(Box::new(WordLearner::default()));
    let cloud_word = |text: &str, syllables: &[&str]| Candidate {
        text: text.into(),
        kind: CandidateKind::Cloud,
        syllables: syllables.iter().map(|s| (*s).to_owned()).collect(),
        reading: None,
        translation: None,
        aux_code: None,
    };
    let has = |engine: &Engine, text: &str| texts_of(engine).iter().any(|t| t == text);
    // 模型把 先 的讀音給成了 xia：敲的 kaixian 切得開、每個音節都是那個字的讀音，按敲的學
    engine.set_input("kaixian");
    engine.commit(&cloud_word("开先", &["kai", "xia"]));
    engine.set_input("kaixian");
    assert!(has(&engine, "开先"));
    let user = engine.learner().user_words().unwrap();
    assert!(
        user.lookup(&["kai", "xian"], false)
            .iter()
            .any(|m| m.exact && m.text == "开先")
    );
    assert!(!user.lookup(&["kai", "xia"], false).iter().any(|m| m.exact));
    // 敲錯了（kaixan 切不開）：模型的讀音每個字都對得上，按模型的學
    engine.set_input("kaixan");
    engine.commit(&cloud_word("开想", &["kai", "xiang"]));
    engine.set_input("kaixiang");
    assert!(has(&engine, "开想"));
    // 敲錯了、模型的讀音又不是這個字的：不學
    engine.set_input("xiangxan");
    engine.commit(&cloud_word("想先", &["xiang", "xia"]));
    let user = engine.learner().user_words().unwrap();
    for reading in [["xiang", "xia"], ["xiang", "xian"]] {
        assert!(
            user.lookup(&reading, false)
                .iter()
                .all(|m| m.text != "想先")
        );
    }
}

#[test]
fn no_predictor_never_requests() {
    let mut engine = engine();
    assert!(!engine.prediction_enabled());
    engine.set_input("kaifa");
    assert_eq!(engine.request_prediction(None, &[]), None);
}

#[test]
fn translation_requests_carry_the_text_and_target_language() {
    use std::sync::{Arc, Mutex};
    struct Recorder(Arc<Mutex<Vec<PredictionRequest>>>);
    impl Predictor for Recorder {
        fn policy(&self) -> PredictionPolicy {
            PredictionPolicy::default()
        }
        fn submit(&mut self, request: PredictionRequest) {
            self.0.lock().unwrap().push(request);
        }
        fn poll(&mut self) -> Option<Prediction> {
            None
        }
    }
    let sent = Arc::new(Mutex::new(Vec::new()));
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_predictor(Box::new(Recorder(sent.clone())));
    assert!(engine.request_translation("  ").is_none());
    let sequence = engine.request_translation("我想去吃饭").unwrap();
    let requests = sent.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].sequence, sequence);
    assert_eq!(requests[0].kind, PredictionKind::Translate);
    assert_eq!(requests[0].text, "我想去吃饭");
    assert_eq!(requests[0].target_language, "en");
    drop(requests);
    // 外文選區譯回中文
    engine.request_translation("I want to eat.").unwrap();
    assert_eq!(sent.lock().unwrap()[1].target_language, "zh");
}

#[test]
fn bare_question_restores_punctuation_once_in_each_mode() {
    for (english, full_width, expected) in [
        (false, false, "?"),
        (false, true, "？"),
        (true, false, "?"),
        (true, true, "?"),
    ] {
        let mut engine = engine();
        engine.set_full_width_punctuation(full_width);
        engine.push('?');
        assert_eq!(
            engine.restore_bare_question(english).as_deref(),
            Some(expected)
        );
        assert!(engine.composition().is_empty());
        assert_eq!(engine.history().text(), expected);
        assert_eq!(engine.passthrough_pending, expected);
        assert_eq!(engine.restore_bare_question(english), None);
        assert_eq!(engine.history().text(), expected);
        assert_eq!(engine.passthrough_pending, expected);
    }
}

#[test]
fn restoring_question_preserves_other_compositions() {
    for input in ["", "nihao", "?nihao"] {
        let mut engine = engine();
        engine.set_input(input);
        assert_eq!(engine.restore_bare_question(false), None);
        assert_eq!(engine.composition().text(), input);
        assert!(engine.history().text().is_empty());
        assert!(engine.passthrough_pending.is_empty());
    }
}
