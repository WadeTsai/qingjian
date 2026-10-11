//! 英文模式與中英混輸。

use super::*;

#[test]
fn english_word_ranks_first_when_input_is_unlikely_pinyin() {
    // 詞頻是 Zipf×1000，與產品 english.tsv 同一尺度
    let words = WordList::parse(
        "hello\thello\t4720\nchina\tchina\t5100\nGitHub\tgithub\t3180\nkey\tkey\t5120\n",
    )
    .unwrap();
    let mut engine = engine().with_english(words);

    engine.set_input("hello"); // he l… l… o：中間有聲母縮寫
    let all: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert_eq!(all[0], "hello");

    engine.set_input("github"); // gi 不是音節 → 切不動
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "GitHub");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::English);
    let word = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&word), "GitHub");
    assert!(engine.composition().is_empty());

    // china 是乾淨的 chi na，中文候選（詞庫裡沒有就只有英文）排前；這裡詞庫沒有 chi na，英文仍在第一位
    engine.set_input("china");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "china");
}

#[test]
fn usage_meter_counts_hanzi_words_and_english_words_per_commit() {
    let recorded = Arc::new(Mutex::new(Vec::new()));
    let mut engine = engine().with_usage_meter(Box::new(MemoryMeter(recorded.clone())));
    engine.set_input("kaifa");
    let kaifa = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(kaifa.text, "开发");
    engine.commit(&kaifa);
    // 拼音回車不算英文詞，像英文的字母串才算
    engine.set_input("hao");
    engine.take_raw();
    engine.set_input("gist");
    engine.take_raw();
    let recorded = recorded.lock().unwrap();
    assert_eq!(
        recorded[0],
        Usage {
            hanzi: 2,
            words: 1,
            english_words: 0,
            commits: 1
        }
    );
    assert_eq!(recorded[1].english_words, 0);
    assert_eq!(recorded[1].commits, 1);
    assert_eq!(recorded[2].english_words, 1);
    assert_eq!(recorded.len(), 3);
}

#[test]
fn english_word_yields_to_a_chinese_word_the_user_keeps_choosing() {
    let dictionary = Dictionary::parse("可以\tke yi\t9000\n客运\tke yun\t100\n").unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(WordList::parse("key\tkey\t5120\n").unwrap())
        .with_learner(Box::new(CountingLearner(HashMap::new())));
    let first_two = |engine: &Engine| {
        let all = texts_of(engine);
        (all[0].clone(), all[1].clone())
    };
    let pick = |engine: &mut Engine, text: &str| {
        engine.set_input("key");
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == text)
            .unwrap();
        engine.commit(&candidate);
    };
    // 開了中文優先：ke'y 再不像話，中文詞也在前、英文第二
    engine.set_chinese_first(true);
    engine.set_input("key");
    assert_eq!(first_two(&engine), ("可以".into(), "key".into()));
    // 預設關：末尾落單一個字母、拼音不像話，英文詞在前
    engine.set_chinese_first(false);
    engine.set_input("key");
    assert_eq!(first_two(&engine), ("key".into(), "可以".into()));
    // 這段字母下選過一次 可以：中文在前，英文退到第二
    pick(&mut engine, "可以");
    engine.set_input("key");
    assert_eq!(first_two(&engine), ("可以".into(), "key".into()));
    // 之後選英文詞的次數反超：英文回到第一
    pick(&mut engine, "key");
    pick(&mut engine, "key");
    engine.set_input("key");
    assert_eq!(first_two(&engine), ("key".into(), "可以".into()));
}

#[test]
fn short_all_caps_acronym_yields_to_chinese() {
    // `mp` 整段只有兩個字母、英文寫法又是全大寫縮寫（MP）：幾乎總是在打 門票，讓中文先，
    // 英文詞仍在候選裡只是退到後面。超過兩個字母的正文英文（hello / cargo）不受這條影響——
    // 曾經試過按詞頻一刀切，凍結日誌回放實測英文首選從 82.5% 掉到 50.9%，cargo / rust 全被整句擠掉。
    let dictionary = Dictionary::parse("门票\tmen piao\t5000\n买票\tmai piao\t3000\n").unwrap();
    let words = WordList::parse("MP\tmp\t4290\nhello\thello\t4720\ncargo\tcargo\t5000\n").unwrap();
    let mut engine = Engine::new(dictionary).with_english(words);

    engine.set_input("mp");
    let all = texts_of(&engine);
    assert_eq!(all[0], "门票");
    assert!(
        all.iter().any(|t| t == "MP"),
        "英文词仍在候选里，只是让到后面"
    );
    assert!(all.iter().position(|t| t == "MP").unwrap() > 0);

    // 三個字母以上的正文英文：拼音不像話時照舊排第一
    engine.set_input("hello");
    assert_eq!(texts_of(&engine)[0], "hello");
    engine.set_input("cargo");
    assert_eq!(texts_of(&engine)[0], "cargo");
}

#[test]
fn learned_english_word_keeps_first_place_over_the_two_letter_acronym_rule() {
    // `ok` / `pc` / `ll` 同樣滿足「兩個字母的全大寫縮寫」，一刀切會把它們一起翻成中文；
    // 而使用者**選過**的英文詞要照舊排第一 —— 選過 OK，下次敲 `ok` 就該還是 OK 在前。
    let dictionary = Dictionary::parse("哦\to\t5000\n门票\tmen piao\t5000\n").unwrap();
    let words = WordList::parse("OK\tok\t5140\nMP\tmp\t4290\n").unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(words)
        .with_learner(Box::new(CountingLearner(HashMap::new())));
    let pick = |engine: &mut Engine, input: &str, text: &str| {
        engine.set_input(input);
        let candidate = engine
            .query()
            .unwrap()
            .candidates
            .items
            .into_iter()
            .find(|c| c.text == text)
            .unwrap();
        engine.commit(&candidate);
    };

    // 沒選過時規則照舊生效：兩個字母的全大寫縮寫讓中文先。
    engine.set_input("mp");
    assert_eq!(texts_of(&engine)[0], "门票");
    engine.set_input("ok");
    assert_eq!(texts_of(&engine)[0], "哦");

    // 選過一次 OK：規則不再壓過學習記錄，`ok` 回到英文第一。
    pick(&mut engine, "ok", "OK");
    engine.set_input("ok");
    assert_eq!(texts_of(&engine)[0], "OK");

    // `mp` 同理：選過 MP 之後它也從中文切回英文第一。
    pick(&mut engine, "mp", "MP");
    engine.set_input("mp");
    assert_eq!(texts_of(&engine)[0], "MP");
}

#[test]
fn hyphen_turns_the_buffer_into_a_raw_english_segment() {
    let mut engine = engine();
    engine.set_input("no");
    assert!(!engine.raw_mode());
    engine.push('-');
    assert!(engine.raw_mode());
    engine.push('w');
    engine.push('a');
    engine.push('y');
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items.len(), 1);
    assert_eq!(query.candidates.items[0].text, "no-way");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::English);
    assert_eq!(query.tail, "no-way");
    let raw = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&raw), "no-way");
    assert!(engine.composition().is_empty());
    // 直輸段不發雲聯想
    engine.set_input("a-b");
    assert_eq!(engine.request_prediction(None, &[]), None);
    // 表示式與問字模式優先
    engine.set_input("v1-2");
    assert!(engine.expression_mode() && !engine.raw_mode());
}

#[test]
fn english_completions_appear_when_pinyin_is_unlikely() {
    // Zipf×1000：company 5.6 / compare 4.45 / compass 3.74 都過補全門檻 3.5
    let words = WordList::parse(
            "company\tcompany\t5600\ncompare\tcompare\t4450\ncompass\tcompass\t3740\ncomma\tcomma\t2900\nxian\txian\t4500\nxiangkai\txiangkai\t10\n",
        )
        .unwrap();
    let mut engine = engine().with_english(words);
    // compa 切成 co'm'pa，不像拼音：補全排最前，最多三條、按詞頻
    engine.set_input("compa");
    let all = texts_of(&engine);
    assert_eq!(&all[..3], ["company", "compare", "compass"]);
    // xian 是乾淨的拼音：只有精確詞 xian，不補全
    engine.set_input("xian");
    let all = texts_of(&engine);
    assert_eq!(all.iter().filter(|t| t.starts_with("xian")).count(), 1);
    // 太短的字首不補全
    engine.set_input("co");
    assert!(!texts_of(&engine).contains(&"company".to_owned()));
    // 第一個字母就切不動的（i 不是任何音節的開頭）也要出補全
    engine.set_input("impo");
    assert!(engine.query().is_err());
    let words = WordList::parse("important\timportant\t4800\nimport\timport\t4600\n").unwrap();
    let mut fresh = Engine::new(Dictionary::parse(SAMPLE).unwrap()).with_english(words);
    fresh.set_input("impo");
    assert_eq!(texts_of(&fresh), ["important", "import"]);
}

#[test]
fn english_in_chinese_off_drops_words_and_completions_but_not_english_mode() {
    let words =
        WordList::parse("company\tcompany\t5600\ncompare\tcompare\t4450\nhello\thello\t1000\n")
            .unwrap();
    let mut engine = engine().with_english(words);
    engine.set_english_in_chinese(false);
    engine.set_input("compa");
    assert!(!texts_of(&engine).iter().any(|t| t.starts_with("comp")));
    engine.set_input("hello");
    assert!(!texts_of(&engine).contains(&"hello".to_owned()));
    engine.set_english_mode(true);
    engine.set_input("comp");
    assert_eq!(texts_of(&engine), ["company", "compare"]);
}

#[test]
fn english_mode_suggests_from_the_word_list_and_keeps_the_typed_text() {
    let words = WordList::parse(
        "company\tcompany\t900\ncompare\tcompare\t500\nhello\thello\t1000\nhelp\thelp\t700\n",
    )
    .unwrap();
    let submitted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut engine = engine()
        .with_english(words)
        .with_learner(Box::new(CountingLearner(HashMap::new())))
        .with_predictor(Box::new(EchoPredictor {
            submitted: submitted.clone(),
            replies: Vec::new(),
            sentence: false,
        }));
    engine.set_english_mode(true);
    assert!(engine.english_mode());
    // 大小寫跟著敲的走，marked text 就是敲的字母，沒有拼音切分
    engine.set_input("Comp");
    let query = engine.query().unwrap();
    assert_eq!(texts_of(&engine), ["Company", "Compare"]);
    assert_eq!(query.marked_text(), "Comp");
    assert!(query.segmentations.is_empty());
    // 數字進緩衝區也只是沒候選
    engine.set_input("foo1");
    assert!(texts_of(&engine).is_empty());
    assert_eq!(engine.query().unwrap().marked_text(), "foo1");
    // 英文模式不發雲聯想
    engine.set_input("comp");
    assert_eq!(engine.request_prediction(None, &[]), None);
    assert!(submitted.borrow().is_empty());
    // 選中的詞記次數，下次同樣的字首它靠前
    let compare = engine.query().unwrap().candidates.items[1].clone();
    assert_eq!(engine.commit(&compare), "compare");
    assert!(engine.composition().is_empty());
    engine.set_input("comp");
    assert_eq!(texts_of(&engine), ["compare", "company"]);
    // 拼錯一個字母也有候選；回車原樣上屏敲的字母
    engine.set_input("helo");
    assert_eq!(texts_of(&engine), ["hello", "help"]);
    assert_eq!(engine.take_raw(), "helo");
    // emoji 排在所有詞後面，不擋上下鍵選詞
    let table = EmojiTable::parse("help\t🆘\n").unwrap();
    let mut engine = engine.with_emoji(table);
    engine.set_english_mode(true);
    engine.set_input("helo");
    assert_eq!(texts_of(&engine), ["hello", "help", "🆘"]);
    // 離開英文模式後同一串又按拼音算
    engine.set_english_mode(false);
    engine.set_input("comp");
    assert!(!engine.query().unwrap().segmentations.is_empty());
}

#[test]
fn raw_committed_english_words_are_learned_and_come_back_as_candidates() {
    use std::cell::RefCell;
    use std::rc::Rc;

    #[derive(Default)]
    struct EnglishLearner {
        words: Vec<String>,
        list: Option<WordList>,
    }
    impl Learner for EnglishLearner {
        fn record(&mut self, _candidate: &Candidate) {}
        fn weight(&self, _text: &str) -> u32 {
            0
        }
        fn learn_english(&mut self, word: &str) {
            self.words.push(word.to_owned());
            let tsv: String = self
                .words
                .iter()
                .map(|w| format!("{w}\t{w}\t1\n"))
                .collect();
            self.list = WordList::parse(&tsv).ok();
        }
        fn user_english(&self) -> Option<&WordList> {
            self.list.as_ref()
        }
    }
    let _ = Rc::new(RefCell::new(()));
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_learner(Box::new(EnglishLearner::default()));
    // 隨包詞表裡沒有 gist：第一次只有拼音候選，回車原樣上屏
    engine.set_input("gist");
    let first = engine.query().unwrap();
    assert!(
        first
            .candidates
            .items
            .iter()
            .all(|c| c.kind != CandidateKind::English)
    );
    assert_eq!(engine.take_raw(), "gist");
    // 第二次 gist 就是英文候選，而且排第一（拼音不像話）
    engine.set_input("gist");
    let second = engine.query().unwrap();
    assert_eq!(second.candidates.items[0].text, "gist");
    assert_eq!(second.candidates.items[0].kind, CandidateKind::English);
    // 能切成完整拼音的串回車不算英文詞
    engine.set_input("hao");
    assert_eq!(engine.take_raw(), "hao");
    engine.set_input("hao");
    let hao = engine.query().unwrap();
    assert!(
        hao.candidates
            .items
            .iter()
            .all(|c| c.kind != CandidateKind::English)
    );
    // 英文模式下直通的詞也學
    engine.set_english_mode(true);
    engine.set_input("wo");
    assert_eq!(engine.take_raw(), "wo");
    engine.set_english_mode(false);
    engine.set_input("wo");
    let wo = engine.query().unwrap();
    assert!(
        wo.candidates
            .items
            .iter()
            .any(|c| c.kind == CandidateKind::English && c.text == "wo")
    );
}

/// 句末的英文詞：`kaifarust` → 開發rust 排第一，拼音行是 `kai'fa'rust`，上屏吃掉整段並把 rust 記進個人英文詞表。
#[test]
fn english_word_at_the_end_of_pinyin_joins_the_sentence() {
    let words =
        WordList::parse("rust\trust\t3740\nID\tid\t4610\nto\tto\t7430\nfan\tfan\t4500\n").unwrap();
    let mut engine = engine().with_english(words);

    engine.set_input("kaifarust");
    let query = engine.query().unwrap();
    let first = query.candidates.items[0].clone();
    assert_eq!(first.text, "开发rust");
    assert_eq!(first.kind, CandidateKind::Sentence);
    assert_eq!(first.syllables, ["kai", "fa", "rust"]);
    assert_eq!(query.tail, "rust");
    assert_eq!(query.marked_text(), "kai'fa'rust");
    // 頭段的詞照常出（逐詞上屏也行）
    assert!(query.candidates.items.iter().any(|c| c.text == "开发"));
    assert_eq!(engine.commit(&first), "开发rust");
    assert!(engine.composition().is_empty());

    // 兩個字母的尾段只認縮寫詞：ID 行，to 不行（`kaifato` 按拼音讀）
    engine.set_input("kaifaid");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发ID");
    engine.set_input("kaifato");
    let all: Vec<String> = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .map(|c| c.text)
        .collect();
    assert!(!all.iter().any(|t| t.ends_with("to")), "{all:?}");

    // 尾段本身是合法拼音又不到四個字母（`fan`）：就是拼音，開飯 照舊
    engine.set_input("kaifan");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开饭");
}

/// 尾段也是合法拼音時兩種讀法比分：`wodedatabase` 英文贏（拼音讀法是四個散字），拼音讀法排第二；
/// `womenqubeijing` 拼音贏（北京 是常用詞），輸了的英文讀法不出。
#[test]
fn pinyin_like_english_tail_competes_with_the_plain_reading() {
    const DICT: &str = "我\two\t900000\n的\tde\t800000\n我的\two de\t500000\n大\tda\t50000\n塔\tta\t3000\n巴\tba\t3000\n瑟\tse\t500\n\
        我们\two men\t400000\n去\tqu\t300000\n北京\tbei jing\t200000\n北\tbei\t20000\n京\tjing\t10000\n";
    let words = WordList::parse("database\tdatabase\t4310\nBeijing\tbeijing\t4500\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap()).with_english(words);

    engine.set_input("wodedatabase");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "我的database");
    // 拼音讀法第二（末尾 se 被敲錯邊讀成 的，散字路徑本來就隨便）
    assert!(query.candidates.items[1].text.starts_with("我的大塔巴"));
    assert_eq!(query.candidates.items[1].kind, CandidateKind::Sentence);
    assert_eq!(query.marked_text(), "wo'de'database");

    engine.set_input("womenqubeijing");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "我们去北京");
    assert!(
        !query
            .candidates
            .items
            .iter()
            .any(|c| c.text.ends_with("Beijing"))
    );
    assert_eq!(query.marked_text(), "wo'men'qu'bei'jing");

    // 上屏英文贏了的整句：吃掉整段，英文詞是最後一個詞
    engine.set_input("wodedatabase");
    let mixed = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(mixed.syllables, ["wo", "de", "database"]);
    assert_eq!(engine.commit(&mixed), "我的database");
    assert!(engine.composition().is_empty());
}

/// 個人 n-gram 再肥也不該左右「混輸還是純拼音」：頭段只覆蓋一部分音節，整段讀法多出來的那幾個音節
/// 會白拿一份按長度累積的個人加成，使用者打得越多混輸越打不出來。比分只看靜態模型加敲錯代價。
#[test]
fn personal_ngram_does_not_decide_between_mixed_and_plain_readings() {
    const DICT: &str = "我\two\t900000\n的\tde\t800000\n我的\two de\t500000\n大\tda\t50000\n塔\tta\t3000\n巴\tba\t3000\n瑟\tse\t500\n";

    struct NgramLearner(crate::sentence::UserNgram);

    impl Learner for NgramLearner {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn user_ngram(&self) -> Option<&crate::sentence::UserNgram> {
            Some(&self.0)
        }
    }

    let mut ngram = crate::sentence::UserNgram::default();
    // 把純拼音那條讀法的每一步都記成使用者的老習慣
    for (previous, word) in [
        (None, "我的"),
        (Some("我的"), "大"),
        (Some("大"), "塔"),
        (Some("塔"), "巴"),
        (Some("巴"), "瑟"),
    ] {
        ngram.record_times(
            crate::sentence::Context {
                previous,
                earlier: None,
            },
            word,
            5000,
        );
    }

    let words = WordList::parse("database\tdatabase\t4310\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap())
        .with_english(words)
        .with_learner(Box::new(NgramLearner(ngram)));

    engine.set_input("wodedatabase");
    let first = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(first.text, "我的database", "个人 n-gram 不该把混输压下去");
}

/// 英文詞頻只問隨包表：個人英文表存的是使用次數，當 Zipf×1000 讀會一律塌到兜底值
/// （`English` 用過兩次 → Zipf 0.002 → 兜底 3.0，真實 5.19），等於用過一次反而更難打出來。
#[test]
fn personal_english_usage_count_does_not_replace_the_shipped_frequency() {
    const DICT: &str = "我\two\t900000\n的\tde\t800000\n我的\two de\t500000\n大\tda\t50000\n塔\tta\t3000\n巴\tba\t3000\n瑟\tse\t500\n";

    struct PersonalEnglish(WordList);

    impl Learner for PersonalEnglish {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn user_english(&self) -> Option<&WordList> {
            Some(&self.0)
        }
    }

    // 個人表裡是「用過 1 次」，隨包表裡是 Zipf 5.19。個人表排在前面，按 Zipf×1000 讀就是 0.001，
    // 會被夾到兜底的 Zipf 3，白丟 5 nat
    let personal = WordList::parse("database\tdatabase\t1\n").unwrap();
    let shipped = WordList::parse("database\tdatabase\t5190\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(DICT).unwrap())
        .with_english(shipped)
        .with_learner(Box::new(PersonalEnglish(personal)));

    let tail = engine
        .split_english_tail("wodedatabase")
        .expect("末尾是英文词");
    let expected = (5.19 - 9.0) * std::f64::consts::LN_10;
    assert!(
        (tail.log_prob - expected).abs() < 1e-9,
        "log_prob {} 不是随包表的 Zipf 5.19（{expected}）",
        tail.log_prob,
    );

    engine.set_input("wodedatabase");
    let first = engine.query().unwrap().candidates.items[0].clone();
    assert_eq!(first.text, "我的database");
}

/// 打不出來時原樣上屏的整串（`kaifarust`）不學成英文詞，學進去了也不許堵掉混輸：
/// 它能讀成「拼音頭 + 英文尾」，把整串當英文詞會讓 `split_english_tail` 一開頭就退出，
/// 於是越打不出來學得越多、越學越打不出來。
#[test]
fn a_raw_committed_mixed_input_does_not_block_the_english_tail() {
    #[derive(Default)]
    struct EnglishLearner {
        words: Vec<String>,
        list: Option<WordList>,
    }

    impl Learner for EnglishLearner {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn learn_english(&mut self, word: &str) {
            self.words.push(word.to_owned());
            let tsv: String = self
                .words
                .iter()
                .map(|w| format!("{w}\t{w}\t1\n"))
                .collect();
            self.list = WordList::parse(&tsv).ok();
        }

        fn user_english(&self) -> Option<&WordList> {
            self.list.as_ref()
        }
    }

    let words = WordList::parse("rust\trust\t3740\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_english(words)
        .with_learner(Box::new(EnglishLearner::default()));

    // 原樣上屏整串：不該被記成英文詞
    engine.set_input("kaifarust");
    assert_eq!(engine.take_raw(), "kaifarust");
    engine.set_input("kaifarust");
    let query = engine.query().unwrap();
    assert!(
        !query
            .candidates
            .items
            .iter()
            .any(|c| c.kind == CandidateKind::English && c.text == "kaifarust"),
        "失败的混输不该进个人英文词表：{:?}",
        query.candidates.items.first(),
    );
    assert_eq!(query.candidates.items[0].text, "开发rust");

    // 就算個人表裡已經有這條（老使用者攢下的），混輸那條路也要照走
    let polluted = WordList::parse("kaifarust\tkaifarust\t1\n").unwrap();
    struct Polluted(WordList);
    impl Learner for Polluted {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn user_english(&self) -> Option<&WordList> {
            Some(&self.0)
        }
    }
    let words = WordList::parse("rust\trust\t3740\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_english(words)
        .with_learner(Box::new(Polluted(polluted)));
    engine.set_input("kaifarust");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "开发rust");
}

/// 英文夾在中間的整串（`kaifarustdaima`）原樣上屏也不學成英文詞。
///
/// 上一個測試那條守衛靠 `split_english_tail`，只看得見句尾的英文；英文在中間時它返回 `None`，
/// 於是整串照學不誤——使用者表裡 `yongdockerbushuhenfangbian` 就是這麼來的，學完還會頂到候選第一位。
/// 這裡靠的是長度上限（[`MAX_ENGLISH_WORD_LETTERS`]）。
#[test]
fn a_long_raw_commit_with_english_in_the_middle_is_not_learned() {
    struct Recorder(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

    impl Learner for Recorder {
        fn record(&mut self, _candidate: &Candidate) {}

        fn weight(&self, _text: &str) -> u32 {
            0
        }

        fn learn_english(&mut self, word: &str) {
            self.0.lock().unwrap().push(word.to_owned());
        }
    }

    let learned = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let words = WordList::parse("rust\trust\t3740\n").unwrap();
    let mut engine = Engine::new(Dictionary::parse(SAMPLE).unwrap())
        .with_english(words)
        .with_learner(Box::new(Recorder(learned.clone())));

    // 英文夾在中間：`split_english_tail` 看不見 rust，只有長度上限攔得住
    engine.set_input("yongdockerbushuhenfangbian");
    assert_eq!(engine.take_raw(), "yongdockerbushuhenfangbian");
    // 短的英文詞照學不誤
    engine.set_input("kubectl");
    assert_eq!(engine.take_raw(), "kubectl");

    assert_eq!(
        *learned.lock().unwrap(),
        vec!["kubectl".to_owned()],
        "只有真正像一个词的才该进个人英文词表"
    );
}
