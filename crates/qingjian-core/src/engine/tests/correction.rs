//! 拼寫糾錯 / 敲錯邊 / 模糊音。

use super::*;

/// 每個音節都合法、整句卻不通的輸入（`meiganxi`）：詞圖裡的敲錯邊把 沒關係 讀出來，作為詞候選插到最前；
/// 上屏按敲的字母消耗、記個人敲錯表，退格重選時退回；原樣說得通（`meiganxie` 沒感謝）時不改。
#[test]
fn typo_edges_in_the_lattice_correct_legal_but_unlikely_pinyin() {
    let dictionary = Dictionary::parse(
        "没关系\tmei guan xi\t800000\n关系\tguan xi\t500000\n没\tmei\t900000\n干\tgan\t200000\n\
             洗\txi\t100000\n美感\tmei gan\t300000\n感谢\tgan xie\t300000\n谢\txie\t50000\n",
    )
    .unwrap();
    let mut engine =
        Engine::new(dictionary).with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_input("meiganxi");
    let query = engine.query().unwrap();
    assert!(query.correction.is_none());
    let first = query.candidates.items[0].clone();
    assert_eq!(first.text, "没关系");
    assert_eq!(first.kind, CandidateKind::Chinese);
    assert_eq!(first.syllables, ["mei", "guan", "xi"]);
    // 詞級候選不帶敲錯變體：原樣的 美感 還在後面
    assert!(query.candidates.items.iter().any(|c| c.text == "美感"));
    assert_eq!(engine.commit(&first), "没关系");
    assert!(engine.composition().is_empty());
    assert_eq!(engine.learner().typo_count("gan", "guan"), 1);
    assert_eq!(engine.learner().choice_weight("meiganxi", "没关系"), 1);
    // 整個刪掉、同一段拼音改選 美感：敲錯的記錄退回
    for _ in 0..3 {
        engine.note_backspace();
    }
    engine.set_input("meiganxi");
    let meigan = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "美感")
        .unwrap();
    engine.commit(&meigan);
    assert_eq!(engine.learner().typo_count("gan", "guan"), 0);
    assert_eq!(engine.composition().text(), "xi");
    // 原樣讀得通：沒 + 感謝 是正常整句，不動
    engine.set_input("meiganxie");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "没感谢");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Sentence);
    // 兩個音節也糾：ganxi → 關係
    engine.set_input("ganxi");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "关系");
    // 太短不糾（不到 4 個字母）
    engine.set_input("gan");
    assert!(
        engine
            .query()
            .unwrap()
            .candidates
            .items
            .iter()
            .all(|c| c.text != "关系")
    );
    // 敲的拼音本身正好是一個詞（按另一種切分）：不許敲錯邊壓過它。jineng 最優切分是 jin eng，詞圖裡讀出 近藤，
    // 但 技能 的音節正好拼成整段輸入
    let dictionary = Dictionary::parse(
        "技能\tji neng\t300000\n近藤\tjin teng\t900000\n近\tjin\t500000\n\
             机\tji\t400000\n能\tneng\t600000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_input("jineng");
    let query = engine.query().unwrap();
    assert_eq!(query.segmentations[0].joined("'"), "jin'eng");
    assert_eq!(query.candidates.items[0].text, "技能");
    assert!(query.candidates.items.iter().all(|c| c.text != "近藤"));
    // 退回原樣的路徑時原樣的整句照出：shude 詞圖裡 是的（shu → shi 相鄰鍵）贏，但 樹德 正好拼成 shude，
    // 於是整句退回 屬的
    let dictionary = Dictionary::parse(
            "树德\tshu de\t500\n是的\tshi de\t5000000\n属\tshu\t100000\n的\tde\t8000000\n是\tshi\t7000000\n",
        )
        .unwrap();
    let mut engine = Engine::new(dictionary);
    engine.set_input("shude");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "属的");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Sentence);
    assert!(query.candidates.items.iter().all(|c| c.text != "是的"));
}

/// 模糊音命中的詞按敲的字母消耗拼音（`zi` 對 `zhi`），不算敲錯。
#[test]
fn fuzzy_hits_consume_the_typed_syllables() {
    let dictionary =
        Dictionary::parse("知识\tzhi shi\t500000\n只是\tzhi shi\t600000\n资\tzi\t1000\n").unwrap();
    let mut engine =
        Engine::new(dictionary).with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_fuzzy(FuzzyRules {
        z_zh: true,
        ..FuzzyRules::default()
    });
    engine.set_input("zishi");
    let zhishi = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "知识")
        .unwrap();
    engine.commit(&zhishi);
    assert!(engine.composition().is_empty());
    assert_eq!(engine.learner().typo_count("zi", "zhi"), 0);
}

/// 接受整段一處編輯的糾正也記個人敲錯表：敲的那段字母對糾正後的音節。
#[test]
fn accepted_whole_string_correction_feeds_the_typo_table() {
    let dictionary = Dictionary::parse(
            "你好吗\tni hao ma\t5000\n你好\tni hao\t9000\n你\tni\t90000\n好\thao\t80000\n吗\tma\t70000\n",
        )
        .unwrap();
    let mut engine =
        Engine::new(dictionary).with_learner(Box::new(CountingLearner(HashMap::new())));
    engine.set_input("nihooma");
    let query = engine.query().unwrap();
    assert_eq!(query.correction.as_ref().unwrap().corrected, "nihaoma");
    let first = query.candidates.items[0].clone();
    engine.commit(&first);
    assert_eq!(engine.learner().typo_count("hoo", "hao"), 1);
    // 只選到 你 就上屏：沒吃到編輯處，不算接受
    engine.set_input("nihooma");
    let ni = engine
        .query()
        .unwrap()
        .candidates
        .items
        .into_iter()
        .find(|c| c.text == "你")
        .unwrap();
    engine.commit(&ni);
    assert_eq!(engine.learner().typo_count("hoo", "hao"), 1);
}

#[test]
fn spelling_correction_fixes_one_edit_and_learns_from_enter() {
    let dictionary = Dictionary::parse(
            "你好吗\tni hao ma\t5000\n你好\tni hao\t9000\n你\tni\t90000\n好\thao\t80000\n吗\tma\t70000\n\
             和\the\t50000\n何\the\t3000\n咯\tlo\t100\n你猴\tni hou\t10\n",
        )
        .unwrap();
    let mut engine = Engine::new(dictionary)
        .with_english(WordList::parse("hello\n").unwrap())
        .with_learner(Box::new(CountingLearner(HashMap::new())));
    // 換錯一個字母：nihooma → nihaoma，候選來自糾正後的拼音，拼音行畫出被改掉的 o
    engine.set_input("nihooma");
    let query = engine.query().unwrap();
    let correction = query.correction.clone().expect("corrected");
    assert_eq!(correction.corrected, "nihaoma");
    assert_eq!(query.candidates.items[0].text, "你好吗");
    let kinds: Vec<(String, MarkedKind)> = query
        .marked_segments()
        .iter()
        .map(|s| (s.text.clone(), s.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("ni'h".to_owned(), MarkedKind::Typed),
            ("o".to_owned(), MarkedKind::Corrected),
            ("ao'ma".to_owned(), MarkedKind::Typed),
        ]
    );
    assert_eq!(query.marked_cursor(), 10);
    // 選糾正後的詞吃掉整段原串，並按原輸入串記選擇
    let first = query.candidates.items[0].clone();
    assert_eq!(engine.commit(&first), "你好吗");
    assert!(engine.composition().is_empty());
    engine.set_input("nihooma");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "你好吗");
    // 相鄰換位
    engine.set_input("nihoama");
    let query = engine.query().unwrap();
    assert_eq!(query.correction.as_ref().unwrap().corrected, "nihaoma");
    // 選字首詞 你好 只吃到 nihoa 之後：換位不改長度，剩 ma
    let nihao = query
        .candidates
        .items
        .iter()
        .find(|c| c.text == "你好")
        .cloned()
        .unwrap();
    engine.commit(&nihao);
    assert_eq!(engine.composition().text(), "ma");
    // 整段是英文詞的不糾（hello 不會變成 和咯）
    engine.set_input("hello");
    assert!(engine.query().unwrap().correction.is_none());
    // 原樣已經說得通的合法簡拼不糾：nhao 是 你好 的簡拼，糾成 nihao 得分一樣、扣掉編輯代價就輸了
    engine.set_input("nhao");
    assert!(engine.query().unwrap().correction.is_none());
    // 末尾單字母只試相鄰換位：nihoa → nihao
    engine.set_input("nihoa");
    assert_eq!(
        engine.query().unwrap().correction.unwrap().corrected,
        "nihao"
    );
    // 短串不糾
    engine.set_input("nih");
    assert!(engine.query().unwrap().correction.is_none());
    // 回車原樣上屏過的串以後不糾
    engine.set_input("nihooma");
    assert!(engine.query().unwrap().correction.is_some());
    assert_eq!(engine.take_raw(), "nihooma");
    engine.set_input("nihooma");
    assert!(engine.query().unwrap().correction.is_none());
}

#[test]
fn transposition_is_corrected_while_the_last_syllable_is_still_unfinished() {
    let dictionary = Dictionary::parse(
        "明天\tming tian\t8000\n明\tming\t12000\n天\ttian\t9000\n米\tmi\t3000\n给\tgei\t5000\n\
         你\tni\t90000\n太\ttai\t4000\n啊\ta\t6000\n",
    )
    .unwrap();
    let mut engine = Engine::new(dictionary);
    // 敲反了 gn 之後還在往下敲：mignt → ming't，候選按前綴出 明天，拼音行劃掉 gn
    engine.set_input("mignt");
    let query = engine.query().unwrap();
    let correction = query.correction.clone().expect("corrected");
    assert_eq!(correction.corrected, "mingt");
    assert!(correction.segmentation.last_is_partial());
    let kinds: Vec<(String, MarkedKind)> = query
        .marked_segments()
        .iter()
        .map(|s| (s.text.clone(), s.kind))
        .collect();
    assert_eq!(
        kinds,
        [
            ("mi".to_owned(), MarkedKind::Typed),
            ("gn".to_owned(), MarkedKind::Corrected),
            ("ng't".to_owned(), MarkedKind::Typed),
        ]
    );
    assert_eq!(query.candidates.items[0].text, "明天");
    // 再敲兩鍵：migntia → ming'tia…，仍是 明天；選它吃掉整段原串
    engine.set_input("migntia");
    let query = engine.query().unwrap();
    assert_eq!(query.correction.as_ref().unwrap().corrected, "mingtia");
    let first = query.candidates.items[0].clone();
    assert_eq!(first.text, "明天");
    assert_eq!(engine.commit(&first), "明天");
    assert!(engine.composition().is_empty());
    // 敲完整個詞：migntian → ming'tian
    engine.set_input("migntian");
    assert_eq!(engine.query().unwrap().candidates.items[0].text, "明天");
}

#[test]
fn commit_alignment_backtracks_over_typo_variants() {
    let engine = engine();
    let syllables =
        |list: &[&str]| -> Vec<String> { list.iter().map(|s| (*s).to_owned()).collect() };
    // pingyin 上屏 拼音：pin 原樣只吃三個字母會剩 gyin，退回來按敲錯變體 ping → pin 吃四個，整段吃光並記敲錯
    let alignment = engine.align("pingyin", &syllables(&["pin", "yin"]));
    assert_eq!(alignment.consumed, 7);
    assert_eq!(alignment.typos, [("ping".to_owned(), "pin".to_owned())]);
    // 原樣、沒打完的字首、字首詞、帶 ' 的輸入照舊
    assert_eq!(
        engine.align("nihao", &syllables(&["ni", "hao"])).consumed,
        5
    );
    assert_eq!(
        engine
            .align("mingt", &syllables(&["ming", "tian"]))
            .consumed,
        5
    );
    assert_eq!(
        engine.align("nihaoma", &syllables(&["ni", "hao"])).consumed,
        5
    );
    assert_eq!(
        engine.align("ni'hao", &syllables(&["ni", "hao"])).consumed,
        6
    );
    // 對不上的候選還是貪心對到哪算哪
    assert_eq!(engine.align("nihao", &syllables(&["ni", "ma"])).consumed, 2);
}

#[test]
fn unfinished_last_syllable_is_not_recorded_as_a_typo() {
    let engine = engine();
    let syllables =
        |list: &[&str]| -> Vec<String> { list.iter().map(|s| (*s).to_owned()).collect() };
    // shijia 選 時間 是 jian 沒敲完：jia 雖是 jian 的敲錯變體，也不進個人敲錯表
    for (input, list) in [
        ("shijia", ["shi", "jian"]),
        ("zhegua", ["zhe", "guan"]),
        ("woxia", ["wo", "xian"]),
    ] {
        let alignment = engine.align(input, &syllables(&list));
        assert_eq!(alignment.consumed, input.len());
        assert!(alignment.typos.is_empty(), "{input}: {:?}", alignment.typos);
    }
}

#[test]
fn fuzzy_rules_add_homophones_behind_exact_hits() {
    // 詞庫裡只有 kai fa 系列加一個 哈；敲 kaiha 沒開 f/h 時只有字首詞 開（開哈 原樣讀得通，詞圖的敲錯邊翻不過它），
    // 開了就出 開發（模糊命中）且覆蓋更多字母排第一
    let mut engine = Engine::new(Dictionary::parse(&format!("{SAMPLE}哈\tha\t50000\n")).unwrap());
    engine.set_input("kaiha");
    let before = texts_of(&engine);
    assert!(!before.contains(&"开发".to_owned()));
    assert!(before.contains(&"开".to_owned()));
    engine.set_fuzzy(FuzzyRules {
        f_h: true,
        ..FuzzyRules::default()
    });
    let after = texts_of(&engine);
    // ha 是字首，換成 fa 字首後 開放（詞頻更高）與 開發 都出，覆蓋更多字母排在 開 前面
    assert_eq!(after[0], "开放");
    assert!(after.contains(&"开发".to_owned()));
    assert!(after.contains(&"开".to_owned()));
    // 整句轉換走同一套寫法：xiangkaiha → 想開發
    engine.set_input("xiangkaiha");
    let query = engine.query().unwrap();
    assert_eq!(query.candidates.items[0].text, "想开发");
    assert_eq!(query.candidates.items[0].kind, CandidateKind::Sentence);
    // 敲對的仍然優先：kaifa 第一位還是 開發，且不重複
    engine.set_input("kaifa");
    let all = texts_of(&engine);
    assert_eq!(all[0], "开发");
    assert_eq!(all.iter().filter(|t| *t == "开发").count(), 1);
}
